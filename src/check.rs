use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::baseline::Baseline;
use crate::cli::{CheckArgs, CppStandard};
use crate::compdb::{self, CompilationDatabase, CompilationUnit};
use crate::config;
use crate::error::AppError;
use crate::report::{
    stage_from_result, BaselineSummary, CoverageMetric, CoverageSummary, Report, ReportStatus,
    StageReport, StageStatus, Summary, TargetInfo, ToolInfo, REPORT_SCHEMA_VERSION,
};
use crate::runner::{run_command, CommandSpec};

const DEFAULT_FUZZ_SECONDS: u64 = 5;
const FUZZ_ENTRYPOINT: &str = "LLVMFuzzerTestOneInput";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Sanitizer {
    Address,
    Undefined,
}

impl Sanitizer {
    fn compiler_name(self) -> &'static str {
        match self {
            Self::Address => "address",
            Self::Undefined => "undefined",
        }
    }

    fn artifact_name(self) -> &'static str {
        match self {
            Self::Address => "asan",
            Self::Undefined => "ubsan",
        }
    }
}

pub fn run(args: CheckArgs) -> Result<Report, AppError> {
    let args = ResolvedCheckArgs::from_cli(args)?;
    let target = if args.coverage && args.file.is_dir() && is_cmake_project(&args.file) {
        CheckTarget::CMakeProject(args.file.clone())
    } else {
        CheckTarget::resolve(&args.file)?
    };

    match target {
        CheckTarget::SourceFile(_) if args.ctest => Err(AppError::CtestRequiresCMakeProject),
        CheckTarget::SourceFile(source) => run_source_file(args, source),
        CheckTarget::CompilationDatabase(_) if args.ctest => {
            Err(AppError::CtestRequiresCMakeProject)
        }
        CheckTarget::CompilationDatabase(database) => run_compilation_database(args, database),
        CheckTarget::CMakeProject(project_dir) => run_cmake_project(args, project_dir),
    }
}

fn run_source_file(args: ResolvedCheckArgs, source: PathBuf) -> Result<Report, AppError> {
    let source = source.canonicalize().unwrap_or(source);
    let artifact_root = args.artifact_dir.clone();
    let build_dir = artifact_root.join("build");
    create_dir(&artifact_root)?;
    create_dir(&build_dir)?;

    if args.fuzz {
        return run_source_file_fuzz(args, source, artifact_root, build_dir);
    }

    let report_paths = resolve_report_paths(&args, &artifact_root);
    let sanitizers = parse_sanitizers(&args.sanitizers)?;
    let timeout = Duration::from_secs(args.timeout_seconds);
    let executable = build_dir.join(executable_name(&source, None));

    let mut stages = Vec::new();

    let compile = compile_stage(
        "compile",
        &args.compiler,
        args.standard.as_flag(),
        &source,
        &executable,
        &[],
        timeout,
    )?;
    let compile_ok = compile.status == StageStatus::Passed;
    stages.push(compile);

    if compile_ok {
        let run_stage = run_executable_stage("run", &executable, timeout)?;
        stages.push(run_stage);
    } else {
        stages.push(StageReport::skipped("run"));
    }

    if args.clang_tidy {
        if compile_ok {
            stages.push(clang_tidy_source_stage(
                &args.clang_tidy_bin,
                args.clang_tidy_checks.as_deref(),
                &source,
                args.standard.as_flag(),
                timeout,
            ));
        } else {
            stages.push(StageReport::skipped("clang_tidy"));
        }
    }

    if sanitizers.is_empty() {
        stages.push(StageReport::skipped("sanitize_compile"));
        stages.push(StageReport::skipped("sanitize_run"));
    } else if stages
        .iter()
        .all(|stage| stage.status != StageStatus::Failed)
    {
        let sanitizer_flags = sanitizer_flags(&sanitizers);
        let sanitizer_executable = build_dir.join(executable_name(&source, Some(&sanitizers)));
        let sanitize_compile = compile_stage(
            "sanitize_compile",
            &args.compiler,
            args.standard.as_flag(),
            &source,
            &sanitizer_executable,
            &sanitizer_flags,
            timeout,
        )?;
        let sanitize_compile_ok = sanitize_compile.status == StageStatus::Passed;
        stages.push(sanitize_compile);

        if sanitize_compile_ok {
            let sanitize_run =
                run_executable_stage("sanitize_run", &sanitizer_executable, timeout)?;
            stages.push(sanitize_run);
        } else {
            stages.push(StageReport::skipped("sanitize_run"));
        }
    } else {
        stages.push(StageReport::skipped("sanitize_compile"));
        stages.push(StageReport::skipped("sanitize_run"));
    }

    append_test_command_stage(&mut stages, &args, source.parent(), timeout);

    let coverage = if args.coverage {
        append_coverage_stages(
            &mut stages,
            &args,
            &source,
            &build_dir,
            &artifact_root,
            timeout,
        )?
    } else {
        None
    };
    let baseline = apply_baseline(&mut stages, &args);
    append_policy_stage(&mut stages, &args, coverage.as_ref(), baseline.as_ref());

    let summary = summarize(&stages, coverage, baseline);
    let status = if summary.failed_stages == 0 {
        ReportStatus::Passed
    } else {
        ReportStatus::Failed
    };

    let report = Report {
        schema_version: REPORT_SCHEMA_VERSION,
        tool: ToolInfo {
            name: "CppGauntlet".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        target: TargetInfo {
            path: source,
            standard: args.standard.as_report_value().to_string(),
            compiler: args.compiler,
        },
        status,
        summary,
        stages,
        report_path: report_paths.json,
        markdown_report_path: report_paths.markdown,
        html_report_path: report_paths.html,
        sarif_report_path: report_paths.sarif,
    };

    write_report_outputs(&report)?;
    Ok(report)
}

fn run_source_file_fuzz(
    args: ResolvedCheckArgs,
    source: PathBuf,
    artifact_root: PathBuf,
    build_dir: PathBuf,
) -> Result<Report, AppError> {
    let report_paths = resolve_report_paths(&args, &artifact_root);
    let timeout = Duration::from_secs(args.timeout_seconds.max(args.fuzz_seconds + 5));
    let fuzz_dir = artifact_root.join("fuzz");
    let fuzz_artifact_dir = fuzz_dir.join("artifacts");
    create_dir(&build_dir)?;
    create_dir(&fuzz_artifact_dir)?;

    let mut stages = Vec::new();
    if args.clang_tidy {
        stages.push(clang_tidy_source_stage(
            &args.clang_tidy_bin,
            args.clang_tidy_checks.as_deref(),
            &source,
            args.standard.as_flag(),
            timeout,
        ));
    }

    append_fuzz_stages(
        &mut stages,
        &args,
        &source,
        &build_dir,
        &fuzz_dir,
        &fuzz_artifact_dir,
        timeout,
    )?;
    append_test_command_stage(&mut stages, &args, source.parent(), timeout);
    let baseline = apply_baseline(&mut stages, &args);
    append_policy_stage(&mut stages, &args, None, baseline.as_ref());

    let summary = summarize(&stages, None, baseline);
    let status = if summary.failed_stages == 0 {
        ReportStatus::Passed
    } else {
        ReportStatus::Failed
    };

    let report = Report {
        schema_version: REPORT_SCHEMA_VERSION,
        tool: ToolInfo {
            name: "CppGauntlet".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        target: TargetInfo {
            path: source,
            standard: args.standard.as_report_value().to_string(),
            compiler: args.compiler,
        },
        status,
        summary,
        stages,
        report_path: report_paths.json,
        markdown_report_path: report_paths.markdown,
        html_report_path: report_paths.html,
        sarif_report_path: report_paths.sarif,
    };

    write_report_outputs(&report)?;
    Ok(report)
}

fn run_project_fuzz(
    args: ResolvedCheckArgs,
    database: CompilationDatabase,
    mut stages: Vec<StageReport>,
    report_paths: ReportPaths,
    target_path: Option<PathBuf>,
    standard: String,
    compiler: String,
) -> Result<Report, AppError> {
    let timeout = Duration::from_secs(args.timeout_seconds.max(args.fuzz_seconds + 5));
    let artifact_root = args.artifact_dir.clone();
    let fuzz_dir = artifact_root.join("fuzz");
    let build_dir = fuzz_dir.join("build");
    let fuzz_artifact_dir = fuzz_dir.join("artifacts");
    create_dir(&build_dir)?;
    create_dir(&fuzz_artifact_dir)?;
    let build_dir = build_dir.canonicalize().unwrap_or(build_dir);
    let fuzz_artifact_dir = fuzz_artifact_dir
        .canonicalize()
        .unwrap_or(fuzz_artifact_dir);

    let mut targets = discover_fuzz_targets(&database);
    stages.push(fuzz_discover_stage(&database, &targets));
    assign_fuzz_target_ids(&mut targets);

    append_clang_tidy_fuzz_target_stages(&mut stages, &targets, &args, &database, timeout);
    append_project_fuzz_stages(
        &mut stages,
        &args,
        &targets,
        &build_dir,
        &fuzz_dir,
        &fuzz_artifact_dir,
        timeout,
    )?;
    append_test_command_stage(&mut stages, &args, database.path.parent(), timeout);
    let baseline = apply_baseline(&mut stages, &args);
    append_policy_stage(&mut stages, &args, None, baseline.as_ref());

    let summary = summarize(&stages, None, baseline);
    let status = if summary.failed_stages == 0 {
        ReportStatus::Passed
    } else {
        ReportStatus::Failed
    };

    let report = Report {
        schema_version: REPORT_SCHEMA_VERSION,
        tool: ToolInfo {
            name: "CppGauntlet".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        target: TargetInfo {
            path: target_path.unwrap_or(database.path),
            standard,
            compiler,
        },
        status,
        summary,
        stages,
        report_path: report_paths.json,
        markdown_report_path: report_paths.markdown,
        html_report_path: report_paths.html,
        sarif_report_path: report_paths.sarif,
    };

    write_report_outputs(&report)?;
    Ok(report)
}

fn run_compilation_database(
    args: ResolvedCheckArgs,
    database: CompilationDatabase,
) -> Result<Report, AppError> {
    run_compilation_database_with_stages(args, database, Vec::new())
}

fn run_compilation_database_with_stages(
    args: ResolvedCheckArgs,
    database: CompilationDatabase,
    mut stages: Vec<StageReport>,
) -> Result<Report, AppError> {
    let artifact_root = args.artifact_dir.clone();
    create_dir(&artifact_root)?;

    let report_paths = resolve_report_paths(&args, &artifact_root);
    let timeout = Duration::from_secs(args.timeout_seconds);
    if args.fuzz {
        return run_project_fuzz(
            args,
            database,
            stages,
            report_paths,
            None,
            "from compilation database".to_string(),
            "from compilation database".to_string(),
        );
    }

    append_compilation_database_compile_stages(&mut stages, &database, timeout)?;
    append_clang_tidy_stages(&mut stages, &database, &args, timeout);
    if !args.coverage {
        let test_dir = database.path.parent();
        append_test_command_stage(&mut stages, &args, test_dir, timeout);
    }
    let coverage = if args.coverage {
        append_compilation_database_coverage_stages(
            &mut stages,
            &args,
            &database,
            &artifact_root,
            timeout,
        )?
    } else {
        None
    };
    let baseline = apply_baseline(&mut stages, &args);
    append_policy_stage(&mut stages, &args, coverage.as_ref(), baseline.as_ref());

    build_and_write_report_with_coverage(
        database.path,
        "from compilation database".to_string(),
        "from compilation database".to_string(),
        stages,
        report_paths,
        coverage,
        baseline,
    )
}

fn run_cmake_project(args: ResolvedCheckArgs, project_dir: PathBuf) -> Result<Report, AppError> {
    let artifact_root = args.artifact_dir.clone();
    let build_dir = artifact_root.join("cmake-build");
    create_dir(&artifact_root)?;

    let report_paths = resolve_report_paths(&args, &artifact_root);
    let timeout = Duration::from_secs(args.timeout_seconds);
    let cmake_stage = cmake_configure_stage(&project_dir, &build_dir, timeout);

    if cmake_stage.status == StageStatus::Failed {
        let mut stages = vec![cmake_stage];
        if args.coverage {
            append_skipped_cmake_coverage_stages(&mut stages);
        }
        let baseline = apply_baseline(&mut stages, &args);
        append_policy_stage(&mut stages, &args, None, baseline.as_ref());
        return build_and_write_report(
            project_dir,
            "from CMake".to_string(),
            "from CMake".to_string(),
            stages,
            report_paths,
            baseline,
        );
    }

    let database = compdb::load_for_target(&build_dir)?;
    let mut stages = vec![cmake_stage];
    let timeout = Duration::from_secs(args.timeout_seconds);
    if args.fuzz {
        return run_project_fuzz(
            args,
            database,
            stages,
            report_paths,
            Some(project_dir),
            "from CMake".to_string(),
            "from CMake".to_string(),
        );
    }

    append_compilation_database_compile_stages(&mut stages, &database, timeout)?;
    append_clang_tidy_stages(&mut stages, &database, &args, timeout);

    if args.ctest {
        if stages
            .iter()
            .all(|stage| stage.status != StageStatus::Failed)
        {
            let build_stage = cmake_build_stage(&build_dir, timeout);
            let build_ok = build_stage.status == StageStatus::Passed;
            stages.push(build_stage);

            if build_ok {
                stages.push(ctest_stage(&build_dir, timeout));
            } else {
                stages.push(StageReport::skipped("ctest"));
            }
        } else {
            stages.push(StageReport::skipped("cmake_build"));
            stages.push(StageReport::skipped("ctest"));
        }
    }

    append_test_command_stage(&mut stages, &args, Some(&project_dir), timeout);

    let coverage = if args.coverage {
        append_cmake_coverage_stages(&mut stages, &args, &project_dir, &artifact_root, timeout)?
    } else {
        None
    };
    let baseline = apply_baseline(&mut stages, &args);
    append_policy_stage(&mut stages, &args, coverage.as_ref(), baseline.as_ref());

    build_and_write_report_with_coverage(
        project_dir,
        "from CMake".to_string(),
        "from CMake".to_string(),
        stages,
        report_paths,
        coverage,
        baseline,
    )
}

#[derive(Debug)]
struct ReportPaths {
    json: PathBuf,
    markdown: Option<PathBuf>,
    html: Option<PathBuf>,
    sarif: Option<PathBuf>,
}

fn resolve_report_paths(args: &ResolvedCheckArgs, artifact_root: &Path) -> ReportPaths {
    ReportPaths {
        json: args
            .report
            .clone()
            .unwrap_or_else(|| artifact_root.join("cppgauntlet-report.json")),
        markdown: args.markdown_report.clone(),
        html: args.html_report.clone(),
        sarif: args.sarif_report.clone(),
    }
}

fn build_and_write_report(
    target_path: PathBuf,
    standard: String,
    compiler: String,
    stages: Vec<StageReport>,
    report_paths: ReportPaths,
    baseline: Option<BaselineSummary>,
) -> Result<Report, AppError> {
    build_and_write_report_with_coverage(
        target_path,
        standard,
        compiler,
        stages,
        report_paths,
        None,
        baseline,
    )
}

fn build_and_write_report_with_coverage(
    target_path: PathBuf,
    standard: String,
    compiler: String,
    stages: Vec<StageReport>,
    report_paths: ReportPaths,
    coverage: Option<CoverageSummary>,
    baseline: Option<BaselineSummary>,
) -> Result<Report, AppError> {
    let summary = summarize(&stages, coverage, baseline);
    let status = if summary.failed_stages == 0 {
        ReportStatus::Passed
    } else {
        ReportStatus::Failed
    };

    let report = Report {
        schema_version: REPORT_SCHEMA_VERSION,
        tool: ToolInfo {
            name: "CppGauntlet".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        target: TargetInfo {
            path: target_path,
            standard,
            compiler,
        },
        status,
        summary,
        stages,
        report_path: report_paths.json,
        markdown_report_path: report_paths.markdown,
        html_report_path: report_paths.html,
        sarif_report_path: report_paths.sarif,
    };

    write_report_outputs(&report)?;
    Ok(report)
}

fn cmake_configure_stage(source_dir: &Path, build_dir: &Path, timeout: Duration) -> StageReport {
    cmake_configure_stage_with_args(
        "cmake_configure",
        source_dir,
        build_dir,
        Vec::new(),
        timeout,
    )
}

fn cmake_coverage_configure_stage(
    source_dir: &Path,
    build_dir: &Path,
    compiler: &str,
    timeout: Duration,
) -> StageReport {
    cmake_configure_stage_with_args(
        "coverage_cmake_configure",
        source_dir,
        build_dir,
        vec![
            format!("-DCMAKE_CXX_COMPILER={compiler}"),
            format!("-DCMAKE_CXX_FLAGS={}", coverage_flags().join(" ")),
            "-DCMAKE_EXE_LINKER_FLAGS=-fprofile-instr-generate".to_string(),
            "-DCMAKE_SHARED_LINKER_FLAGS=-fprofile-instr-generate".to_string(),
        ],
        timeout,
    )
}

fn cmake_configure_stage_with_args(
    name: &str,
    source_dir: &Path,
    build_dir: &Path,
    extra_args: Vec<String>,
    timeout: Duration,
) -> StageReport {
    let mut args = vec![
        "-S".to_string(),
        source_dir.display().to_string(),
        "-B".to_string(),
        build_dir.display().to_string(),
        "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON".to_string(),
    ];
    args.extend(extra_args);
    let spec = CommandSpec::new("cmake").args(args);
    let command = spec.command_line();

    match run_command(spec, timeout) {
        Ok(result) => stage_from_result(name, result, Some(build_dir)),
        Err(error) => StageReport {
            name: name.to_string(),
            status: StageStatus::Failed,
            command,
            exit_code: None,
            timed_out: false,
            warnings: 0,
            errors: 1,
            diagnostics: Vec::new(),
            stdout: String::new(),
            stderr: error.to_string(),
            artifact: Some(build_dir.to_path_buf()),
        },
    }
}

fn cmake_build_stage(build_dir: &Path, timeout: Duration) -> StageReport {
    cmake_build_stage_named("cmake_build", build_dir, timeout)
}

fn cmake_build_stage_named(name: &str, build_dir: &Path, timeout: Duration) -> StageReport {
    let args = vec!["--build".to_string(), build_dir.display().to_string()];
    let spec = CommandSpec::new("cmake").args(args);
    let command = spec.command_line();
    let result = run_command(spec, timeout);
    stage_from_command_result(name, command, result, Some(build_dir))
}

fn ctest_stage(build_dir: &Path, timeout: Duration) -> StageReport {
    ctest_stage_named("ctest", build_dir, None, timeout)
}

fn ctest_stage_named(
    name: &str,
    build_dir: &Path,
    profile_pattern: Option<&Path>,
    timeout: Duration,
) -> StageReport {
    let args = vec![
        "--test-dir".to_string(),
        build_dir.display().to_string(),
        "--output-on-failure".to_string(),
    ];
    let mut spec = CommandSpec::new("ctest").args(args);
    if let Some(profile_pattern) = profile_pattern {
        spec = spec.env("LLVM_PROFILE_FILE", profile_pattern.display().to_string());
    }
    let command = spec.command_line();
    let result = run_command(spec, timeout);
    stage_from_command_result(name, command, result, Some(build_dir))
}

fn stage_from_command_result(
    name: impl Into<String>,
    command: Vec<String>,
    result: Result<crate::runner::CommandResult, AppError>,
    artifact: Option<&Path>,
) -> StageReport {
    let name = name.into();
    match result {
        Ok(result) => stage_from_result(name, result, artifact),
        Err(error) => StageReport {
            name,
            status: StageStatus::Failed,
            command,
            exit_code: None,
            timed_out: false,
            warnings: 0,
            errors: 1,
            diagnostics: Vec::new(),
            stdout: String::new(),
            stderr: error.to_string(),
            artifact: artifact.map(Path::to_path_buf),
        },
    }
}

fn append_compilation_database_compile_stages(
    stages: &mut Vec<StageReport>,
    database: &CompilationDatabase,
    timeout: Duration,
) -> Result<(), AppError> {
    stages.extend(
        database
            .entries
            .iter()
            .map(|unit| compilation_database_compile_stage(unit, timeout))
            .collect::<Result<Vec<_>, _>>()?,
    );

    Ok(())
}

fn append_clang_tidy_stages(
    stages: &mut Vec<StageReport>,
    database: &CompilationDatabase,
    args: &ResolvedCheckArgs,
    timeout: Duration,
) {
    if !args.clang_tidy {
        return;
    }

    let database_dir = database.path.parent().unwrap_or_else(|| Path::new("."));
    if stages
        .iter()
        .any(|stage| stage.status == StageStatus::Failed)
    {
        stages.extend(database.entries.iter().map(|unit| {
            StageReport::skipped(format!("clang_tidy:{}", source_path(unit).display()))
        }));
        return;
    }

    stages.extend(database.entries.iter().map(|unit| {
        clang_tidy_compilation_database_stage(
            unit,
            &args.clang_tidy_bin,
            args.clang_tidy_checks.as_deref(),
            database_dir,
            timeout,
        )
    }));
}

fn append_test_command_stage(
    stages: &mut Vec<StageReport>,
    args: &ResolvedCheckArgs,
    current_dir: Option<&Path>,
    timeout: Duration,
) {
    let Some(command) = args.test_command.as_deref() else {
        return;
    };

    if stages
        .iter()
        .any(|stage| stage.status == StageStatus::Failed)
    {
        stages.push(StageReport::skipped("test_command"));
        return;
    }

    stages.push(test_command_stage(command, current_dir, timeout));
}

fn test_command_stage(command: &str, current_dir: Option<&Path>, timeout: Duration) -> StageReport {
    let mut spec = shell_command_spec(command);
    if let Some(current_dir) = current_dir {
        spec = spec.current_dir(current_dir.to_path_buf());
    }
    let command_line = spec.command_line();
    let result = run_command(spec, timeout);
    stage_from_command_result("test_command", command_line, result, None)
}

fn append_fuzz_stages(
    stages: &mut Vec<StageReport>,
    args: &ResolvedCheckArgs,
    source: &Path,
    build_dir: &Path,
    fuzz_dir: &Path,
    fuzz_artifact_dir: &Path,
    timeout: Duration,
) -> Result<(), AppError> {
    if stages
        .iter()
        .any(|stage| stage.status == StageStatus::Failed)
    {
        stages.push(StageReport::skipped("fuzz_compile"));
        stages.push(StageReport::skipped("fuzz_run"));
        return Ok(());
    }

    let sanitizers = parse_sanitizers(&args.sanitizers)?;
    let executable = build_dir.join(fuzz_executable_name(source));
    let fuzz_compile = compile_stage(
        "fuzz_compile",
        &args.compiler,
        args.standard.as_flag(),
        source,
        &executable,
        &fuzz_flags(&sanitizers),
        timeout,
    )?;
    let fuzz_compile_ok = fuzz_compile.status == StageStatus::Passed;
    stages.push(fuzz_compile);

    if fuzz_compile_ok {
        let default_corpus = fuzz_dir.join("corpus");
        let corpus = selected_fuzz_corpus(&default_corpus, &args.fuzz_corpus);
        for path in &corpus {
            create_dir(path)?;
        }

        stages.push(fuzz_run_stage(
            "fuzz_run",
            &executable,
            fuzz_artifact_dir,
            &corpus,
            args.fuzz_seconds,
            timeout,
        ));
    } else {
        stages.push(StageReport::skipped("fuzz_run"));
    }

    Ok(())
}

fn append_project_fuzz_stages(
    stages: &mut Vec<StageReport>,
    args: &ResolvedCheckArgs,
    targets: &[FuzzTarget],
    build_dir: &Path,
    fuzz_dir: &Path,
    fuzz_artifact_dir: &Path,
    timeout: Duration,
) -> Result<(), AppError> {
    if stages
        .iter()
        .any(|stage| stage.status == StageStatus::Failed)
    {
        stages.extend(targets.iter().flat_map(|target| {
            [
                StageReport::skipped(fuzz_stage_name("fuzz_compile", &target.source)),
                StageReport::skipped(fuzz_stage_name("fuzz_run", &target.source)),
            ]
        }));
        return Ok(());
    }

    let sanitizers = parse_sanitizers(&args.sanitizers)?;
    let fuzz_flags = fuzz_flags(&sanitizers);

    for target in targets {
        let executable = build_dir.join(project_fuzz_executable_name(target));
        let fuzz_compile = project_fuzz_compile_stage(target, &executable, &fuzz_flags, timeout)?;
        let fuzz_compile_ok = fuzz_compile.status == StageStatus::Passed;
        stages.push(fuzz_compile);

        if fuzz_compile_ok {
            let default_corpus = fuzz_dir.join("corpus").join(&target.artifact_id);
            let corpus = selected_fuzz_corpus(&default_corpus, &args.fuzz_corpus);
            for path in &corpus {
                create_dir(path)?;
            }

            let artifact_dir = fuzz_artifact_dir.join(&target.artifact_id);
            create_dir(&artifact_dir)?;
            stages.push(fuzz_run_stage(
                fuzz_stage_name("fuzz_run", &target.source),
                &executable,
                &artifact_dir,
                &corpus,
                args.fuzz_seconds,
                timeout,
            ));
        } else {
            stages.push(StageReport::skipped(fuzz_stage_name(
                "fuzz_run",
                &target.source,
            )));
        }
    }

    Ok(())
}

fn append_clang_tidy_fuzz_target_stages(
    stages: &mut Vec<StageReport>,
    targets: &[FuzzTarget],
    args: &ResolvedCheckArgs,
    database: &CompilationDatabase,
    timeout: Duration,
) {
    if !args.clang_tidy {
        return;
    }

    let database_dir = database.path.parent().unwrap_or_else(|| Path::new("."));
    if stages
        .iter()
        .any(|stage| stage.status == StageStatus::Failed)
    {
        stages.extend(
            targets.iter().map(|target| {
                StageReport::skipped(format!("clang_tidy:{}", target.source.display()))
            }),
        );
        return;
    }

    stages.extend(targets.iter().map(|target| {
        clang_tidy_compilation_database_stage(
            &target.unit,
            &args.clang_tidy_bin,
            args.clang_tidy_checks.as_deref(),
            database_dir,
            timeout,
        )
    }));
}

fn project_fuzz_compile_stage(
    target: &FuzzTarget,
    output: &Path,
    fuzz_flags: &[String],
    timeout: Duration,
) -> Result<StageReport, AppError> {
    let Some((program, args)) = target.unit.arguments.split_first() else {
        return Err(AppError::InvalidCompilationCommand(
            target.unit.file.clone(),
        ));
    };

    let result = run_command(
        CommandSpec::new(program)
            .args(fuzz_compilation_args(
                args,
                &target.source,
                output,
                fuzz_flags,
            ))
            .current_dir(target.unit.directory.clone()),
        timeout,
    )?;

    Ok(stage_from_result(
        fuzz_stage_name("fuzz_compile", &target.source),
        result,
        Some(output),
    ))
}

fn fuzz_run_stage(
    name: impl Into<String>,
    executable: &Path,
    fuzz_artifact_dir: &Path,
    corpus: &[PathBuf],
    seconds: u64,
    timeout: Duration,
) -> StageReport {
    let mut args = vec![
        format!("-max_total_time={seconds}"),
        format!("-artifact_prefix={}/", fuzz_artifact_dir.display()),
    ];
    args.extend(corpus.iter().map(|path| path.display().to_string()));

    let spec = CommandSpec::new(executable.display().to_string()).args(args);
    let command_line = spec.command_line();
    let result = run_command(spec, timeout);
    stage_from_command_result(name, command_line, result, Some(fuzz_artifact_dir))
}

fn selected_fuzz_corpus(default_corpus: &Path, corpus: &[PathBuf]) -> Vec<PathBuf> {
    if corpus.is_empty() {
        vec![default_corpus.to_path_buf()]
    } else {
        corpus.to_vec()
    }
}

#[cfg(unix)]
fn shell_command_spec(command: &str) -> CommandSpec {
    CommandSpec::new("sh").args(["-c", command])
}

#[cfg(windows)]
fn shell_command_spec(command: &str) -> CommandSpec {
    CommandSpec::new("cmd").args(["/C", command])
}

fn append_policy_stage(
    stages: &mut Vec<StageReport>,
    args: &ResolvedCheckArgs,
    coverage: Option<&CoverageSummary>,
    baseline: Option<&BaselineSummary>,
) {
    if args.max_warnings.is_none()
        && args.max_analyzer_findings.is_none()
        && args.min_line_coverage.is_none()
        && args.min_changed_line_coverage.is_none()
        && !args.fail_on_new_diagnostics
    {
        return;
    }

    if stages
        .iter()
        .any(|stage| stage.status == StageStatus::Failed)
    {
        stages.push(StageReport::skipped("policy"));
        return;
    }

    stages.push(policy_stage(stages, args, coverage, baseline));
}

fn policy_stage(
    stages: &[StageReport],
    args: &ResolvedCheckArgs,
    coverage: Option<&CoverageSummary>,
    baseline: Option<&BaselineSummary>,
) -> StageReport {
    let warnings: usize = stages.iter().map(|stage| stage.warnings).sum();
    let analyzer_findings = analyzer_finding_count(stages);
    let mut failures = Vec::new();
    let mut passes = Vec::new();

    if let Some(max_warnings) = args.max_warnings {
        if warnings > max_warnings {
            failures.push(format!(
                "warnings {warnings} exceed configured maximum {max_warnings}"
            ));
        } else {
            passes.push(format!("warnings {warnings} <= {max_warnings}"));
        }
    }

    if let Some(max_analyzer_findings) = args.max_analyzer_findings {
        if analyzer_findings > max_analyzer_findings {
            failures.push(format!(
                "analyzer findings {analyzer_findings} exceed configured maximum {max_analyzer_findings}"
            ));
        } else {
            passes.push(format!(
                "analyzer findings {analyzer_findings} <= {max_analyzer_findings}"
            ));
        }
    }

    if let Some(min_line_coverage) = args.min_line_coverage {
        match coverage {
            Some(coverage) if coverage.lines.percent >= min_line_coverage => {
                passes.push(format!(
                    "line coverage {:.2}% >= {:.2}%",
                    coverage.lines.percent, min_line_coverage
                ));
            }
            Some(coverage) => {
                failures.push(format!(
                    "line coverage {:.2}% is below configured minimum {:.2}%",
                    coverage.lines.percent, min_line_coverage
                ));
            }
            None => {
                failures.push(format!(
                    "line coverage summary is unavailable; configured minimum is {min_line_coverage:.2}%"
                ));
            }
        }
    }

    if let Some(min_changed_line_coverage) = args.min_changed_line_coverage {
        match coverage.and_then(|coverage| coverage.changed_lines.as_ref()) {
            Some(changed_lines) if changed_lines.percent >= min_changed_line_coverage => {
                passes.push(format!(
                    "changed-line coverage {:.2}% >= {:.2}%",
                    changed_lines.percent, min_changed_line_coverage
                ));
            }
            Some(changed_lines) => {
                failures.push(format!(
                    "changed-line coverage {:.2}% is below configured minimum {:.2}%",
                    changed_lines.percent, min_changed_line_coverage
                ));
            }
            None => {
                failures.push(format!(
                    "changed-line coverage summary is unavailable; configured minimum is {min_changed_line_coverage:.2}%"
                ));
            }
        }
    }

    if args.fail_on_new_diagnostics {
        match baseline {
            Some(baseline) if baseline.new_diagnostic_occurrences == 0 => {
                passes.push("new diagnostics 0 <= 0".to_string());
            }
            Some(baseline) => {
                failures.push(format!(
                    "new diagnostics {} exceed baseline allowance 0",
                    baseline.new_diagnostic_occurrences
                ));
            }
            None => {
                failures.push("new diagnostic policy requires a baseline summary".to_string());
            }
        }
    }

    let status = if failures.is_empty() {
        StageStatus::Passed
    } else {
        StageStatus::Failed
    };

    StageReport {
        name: "policy".to_string(),
        status,
        command: Vec::new(),
        exit_code: None,
        timed_out: false,
        warnings: 0,
        errors: failures.len(),
        diagnostics: Vec::new(),
        stdout: passes.join("\n"),
        stderr: failures.join("\n"),
        artifact: None,
    }
}

fn analyzer_finding_count(stages: &[StageReport]) -> usize {
    stages
        .iter()
        .filter(|stage| is_analyzer_stage(&stage.name))
        .map(|stage| stage.diagnostics.len())
        .sum()
}

fn is_analyzer_stage(name: &str) -> bool {
    name == "clang_tidy" || name.starts_with("clang_tidy:")
}

fn apply_baseline(stages: &mut [StageReport], args: &ResolvedCheckArgs) -> Option<BaselineSummary> {
    args.baseline
        .as_ref()
        .map(|baseline| baseline.compare(stages))
}

fn validate_coverage_threshold(value: Option<f64>) -> Result<Option<f64>, AppError> {
    match value {
        Some(value) if value.is_finite() && (0.0..=100.0).contains(&value) => Ok(Some(value)),
        Some(value) => Err(AppError::InvalidCoverageThreshold(value)),
        None => Ok(None),
    }
}

fn parse_changed_lines(values: Vec<String>) -> Result<Vec<ChangedLine>, AppError> {
    let mut changed_lines = Vec::new();
    let mut seen = HashSet::new();

    for value in values {
        let (path, line) = value
            .rsplit_once(':')
            .ok_or_else(|| AppError::InvalidChangedLine(value.clone()))?;
        if path.trim().is_empty() {
            return Err(AppError::InvalidChangedLine(value));
        }

        let line = line
            .parse::<u64>()
            .ok()
            .filter(|line| *line > 0)
            .ok_or_else(|| AppError::InvalidChangedLine(value.clone()))?;
        let changed_line = ChangedLine {
            path: PathBuf::from(path),
            line,
        };

        if seen.insert(changed_line.clone()) {
            changed_lines.push(changed_line);
        }
    }

    Ok(changed_lines)
}

fn read_changed_lines_diff(path: &Path) -> Result<Vec<ChangedLine>, AppError> {
    let contents = fs::read_to_string(path).map_err(|source| AppError::ReadChangedLinesDiff {
        path: path.to_path_buf(),
        source,
    })?;
    parse_changed_lines_diff(path, &contents)
}

fn parse_changed_lines_diff(path: &Path, contents: &str) -> Result<Vec<ChangedLine>, AppError> {
    let mut changed_lines = Vec::new();
    let mut seen = HashSet::new();
    let mut current_file: Option<PathBuf> = None;
    let mut new_line: Option<u64> = None;

    for raw_line in contents.lines() {
        if raw_line.starts_with("diff --git ") {
            current_file = None;
            new_line = None;
            continue;
        }

        if new_line.is_none() && raw_line.starts_with("--- ") {
            continue;
        }

        if let Some(file) = raw_line.strip_prefix("+++ ") {
            current_file = parse_diff_file(file);
            new_line = None;
            continue;
        }

        if raw_line.starts_with("index ")
            || raw_line.starts_with("new file mode ")
            || raw_line.starts_with("deleted file mode ")
            || raw_line.starts_with("similarity index ")
            || raw_line.starts_with("rename from ")
            || raw_line.starts_with("rename to ")
        {
            continue;
        }

        if raw_line.starts_with("@@") {
            let line = parse_diff_hunk_new_start(raw_line).ok_or_else(|| {
                AppError::ParseChangedLinesDiff {
                    path: path.to_path_buf(),
                    reason: format!("invalid hunk header '{raw_line}'"),
                }
            })?;
            new_line = Some(line);
            continue;
        }

        let Some(line) = new_line else {
            continue;
        };
        let Some(file) = &current_file else {
            continue;
        };

        if raw_line.starts_with('+') && !raw_line.starts_with("+++") {
            let changed_line = ChangedLine {
                path: file.clone(),
                line,
            };
            if seen.insert(changed_line.clone()) {
                changed_lines.push(changed_line);
            }
            new_line = Some(line + 1);
        } else if raw_line.starts_with('-') && !raw_line.starts_with("---") {
            continue;
        } else if raw_line.starts_with(' ') {
            new_line = Some(line + 1);
        } else if raw_line.starts_with('\\') {
            continue;
        } else if !raw_line.is_empty() {
            return Err(AppError::ParseChangedLinesDiff {
                path: path.to_path_buf(),
                reason: format!("unexpected diff line '{raw_line}'"),
            });
        }
    }

    Ok(changed_lines)
}

fn parse_diff_file(value: &str) -> Option<PathBuf> {
    let token = value.split_whitespace().next()?;
    if token == "/dev/null" {
        return None;
    }

    Some(PathBuf::from(
        token
            .strip_prefix("b/")
            .or_else(|| token.strip_prefix("a/"))
            .unwrap_or(token),
    ))
}

fn parse_diff_hunk_new_start(value: &str) -> Option<u64> {
    let plus_index = value.find('+')?;
    let after_plus = &value[plus_index + 1..];
    let end = after_plus
        .find(|ch: char| ch == ',' || ch.is_whitespace())
        .unwrap_or(after_plus.len());
    after_plus[..end].parse::<u64>().ok()
}

fn append_unique_changed_lines(target: &mut Vec<ChangedLine>, additional: Vec<ChangedLine>) {
    let mut seen = target.iter().cloned().collect::<HashSet<_>>();
    for changed_line in additional {
        if seen.insert(changed_line.clone()) {
            target.push(changed_line);
        }
    }
}

enum CheckTarget {
    SourceFile(PathBuf),
    CompilationDatabase(CompilationDatabase),
    CMakeProject(PathBuf),
}

impl CheckTarget {
    fn resolve(path: &Path) -> Result<Self, AppError> {
        if !path.exists() {
            return Err(AppError::TargetMissing(path.to_path_buf()));
        }

        if is_compilation_database_file(path) {
            return compdb::load_for_target(path).map(Self::CompilationDatabase);
        }

        if path.is_dir() {
            return match compdb::load_for_target(path) {
                Ok(database) => Ok(Self::CompilationDatabase(database)),
                Err(AppError::CompilationDatabaseMissing(_)) if is_cmake_project(path) => {
                    Ok(Self::CMakeProject(path.to_path_buf()))
                }
                Err(error) => Err(error),
            };
        }

        if path.is_file() {
            return Ok(Self::SourceFile(path.to_path_buf()));
        }

        Err(AppError::UnsupportedCheckTarget(path.to_path_buf()))
    }
}

fn discover_fuzz_targets(database: &CompilationDatabase) -> Vec<FuzzTarget> {
    let mut seen = HashSet::new();
    let mut targets = Vec::new();

    for unit in &database.entries {
        let source = source_path(unit);
        if !seen.insert(source.clone()) {
            continue;
        }

        let Ok(contents) = fs::read_to_string(&source) else {
            continue;
        };
        if contents.contains(FUZZ_ENTRYPOINT) {
            targets.push(FuzzTarget {
                unit: unit.clone(),
                source,
                artifact_id: String::new(),
            });
        }
    }

    targets.sort_by(|left, right| left.source.cmp(&right.source));
    targets
}

fn assign_fuzz_target_ids(targets: &mut [FuzzTarget]) {
    for (index, target) in targets.iter_mut().enumerate() {
        let path_id = sanitize_path_id(&target.source);
        target.artifact_id = format!("{index:03}-{path_id}");
    }
}

fn fuzz_discover_stage(database: &CompilationDatabase, targets: &[FuzzTarget]) -> StageReport {
    let command = vec![
        "cppgauntlet".to_string(),
        "discover-fuzz-targets".to_string(),
        database.path.display().to_string(),
    ];

    if targets.is_empty() {
        return failed_stage(
            "fuzz_discover",
            command,
            &format!(
                "no fuzz targets containing {FUZZ_ENTRYPOINT} were found in {}",
                database.path.display()
            ),
            Some(&database.path),
        );
    }

    StageReport {
        name: "fuzz_discover".to_string(),
        status: StageStatus::Passed,
        command,
        exit_code: Some(0),
        timed_out: false,
        warnings: 0,
        errors: 0,
        diagnostics: Vec::new(),
        stdout: targets
            .iter()
            .map(|target| target.source.display().to_string())
            .collect::<Vec<_>>()
            .join("\n"),
        stderr: String::new(),
        artifact: Some(database.path.clone()),
    }
}

#[derive(Debug)]
struct ResolvedCheckArgs {
    file: PathBuf,
    standard: CppStandard,
    compiler: String,
    sanitizers: String,
    artifact_dir: PathBuf,
    report: Option<PathBuf>,
    markdown_report: Option<PathBuf>,
    html_report: Option<PathBuf>,
    sarif_report: Option<PathBuf>,
    timeout_seconds: u64,
    ctest: bool,
    test_command: Option<String>,
    clang_tidy: bool,
    clang_tidy_bin: String,
    clang_tidy_checks: Option<String>,
    coverage: bool,
    llvm_cov_bin: String,
    llvm_profdata_bin: String,
    coverage_sources: Vec<PathBuf>,
    coverage_objects: Vec<PathBuf>,
    changed_lines: Vec<ChangedLine>,
    fuzz: bool,
    fuzz_seconds: u64,
    fuzz_corpus: Vec<PathBuf>,
    baseline: Option<Baseline>,
    max_warnings: Option<usize>,
    max_analyzer_findings: Option<usize>,
    min_line_coverage: Option<f64>,
    min_changed_line_coverage: Option<f64>,
    fail_on_new_diagnostics: bool,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ChangedLine {
    path: PathBuf,
    line: u64,
}

#[derive(Clone, Debug)]
struct FuzzTarget {
    unit: CompilationUnit,
    source: PathBuf,
    artifact_id: String,
}

impl ResolvedCheckArgs {
    fn from_cli(args: CheckArgs) -> Result<Self, AppError> {
        let config = config::load_config(args.config.as_deref())?;
        let config_standard = config.standard()?;
        let config_sanitizers = config.sanitizers_csv();
        let config_report = config.report_path();
        let config_markdown_report = config.markdown_report_path();
        let config_html_report = config.html_report_path();
        let config_sarif_report = config.sarif_report_path();
        let config_ctest = config.ctest_enabled();
        let config_test_command = config.test_command();
        let config_clang_tidy = config.clang_tidy_enabled();
        let config_clang_tidy_bin = config.clang_tidy_bin();
        let config_clang_tidy_checks = config.clang_tidy_checks();
        let config_coverage = config.coverage_enabled();
        let config_llvm_cov_bin = config.llvm_cov_bin();
        let config_llvm_profdata_bin = config.llvm_profdata_bin();
        let config_coverage_sources = config.coverage_sources();
        let config_coverage_objects = config.coverage_objects();
        let config_fuzz = config.fuzz_enabled();
        let config_fuzz_seconds = config.fuzz_seconds();
        let config_fuzz_corpus = config.fuzz_corpus();
        let config_baseline_path = config.baseline_path();
        let config_max_warnings = config.max_warnings();
        let config_max_analyzer_findings = config.max_analyzer_findings();
        let config_min_line_coverage = config.min_line_coverage();
        let config_min_changed_line_coverage = config.min_changed_line_coverage();
        let config_changed_lines = config.changed_lines();
        let config_changed_lines_diff = config.changed_lines_diff();
        let config_fail_on_new_diagnostics = config.fail_on_new_diagnostics();
        let max_analyzer_findings = args.max_analyzer_findings.or(config_max_analyzer_findings);
        let cli_requested_clang_tidy = args.clang_tidy
            || args.clang_tidy_bin.is_some()
            || args.clang_tidy_checks.is_some()
            || max_analyzer_findings.is_some();
        let coverage_sources = if args.coverage_sources.is_empty() {
            config_coverage_sources
        } else {
            args.coverage_sources.clone()
        };
        let coverage_objects = if args.coverage_objects.is_empty() {
            config_coverage_objects
        } else {
            args.coverage_objects.clone()
        };
        let changed_line_values = if args.changed_lines.is_empty() {
            config_changed_lines
        } else {
            args.changed_lines.clone()
        };
        let mut changed_lines = parse_changed_lines(changed_line_values)?;
        let changed_lines_diff = args.changed_lines_diff.or(config_changed_lines_diff);
        if let Some(path) = changed_lines_diff {
            append_unique_changed_lines(&mut changed_lines, read_changed_lines_diff(&path)?);
        }
        let cli_requested_coverage = args.coverage
            || args.llvm_cov_bin.is_some()
            || args.llvm_profdata_bin.is_some()
            || !coverage_sources.is_empty()
            || !coverage_objects.is_empty()
            || !changed_lines.is_empty();
        let coverage = cli_requested_coverage || config_coverage.unwrap_or(false);
        let fuzz = args.fuzz || config_fuzz.unwrap_or(false);
        if fuzz && coverage {
            return Err(AppError::FuzzCoverageUnsupported);
        }
        let fuzz_seconds = args
            .fuzz_seconds
            .or(config_fuzz_seconds)
            .unwrap_or(DEFAULT_FUZZ_SECONDS);
        if fuzz_seconds == 0 {
            return Err(AppError::InvalidFuzzSeconds(fuzz_seconds));
        }
        let fuzz_corpus = if args.fuzz_corpus.is_empty() {
            config_fuzz_corpus
        } else {
            args.fuzz_corpus.clone()
        };
        let baseline_path = args.baseline.or(config_baseline_path);
        let fail_on_new_diagnostics =
            args.fail_on_new_diagnostics || config_fail_on_new_diagnostics.unwrap_or(false);
        if fail_on_new_diagnostics && baseline_path.is_none() {
            return Err(AppError::BaselineRequired);
        }
        let baseline = baseline_path.as_deref().map(Baseline::load).transpose()?;
        let min_line_coverage =
            validate_coverage_threshold(args.min_line_coverage.or(config_min_line_coverage))?;
        let min_changed_line_coverage = validate_coverage_threshold(
            args.min_changed_line_coverage
                .or(config_min_changed_line_coverage),
        )?;

        Ok(Self {
            file: args.file,
            standard: args
                .standard
                .or(config_standard)
                .unwrap_or(CppStandard::Cxx20),
            compiler: args
                .compiler
                .or(config.compiler)
                .unwrap_or_else(|| "clang++".to_string()),
            sanitizers: args
                .sanitizers
                .or(config_sanitizers)
                .unwrap_or_else(|| "address,undefined".to_string()),
            artifact_dir: args
                .artifact_dir
                .or(config.artifact_dir)
                .unwrap_or_else(|| PathBuf::from(".cppgauntlet")),
            report: args.report.or(config_report),
            markdown_report: args.markdown_report.or(config_markdown_report),
            html_report: args.html_report.or(config_html_report),
            sarif_report: args.sarif_report.or(config_sarif_report),
            timeout_seconds: args
                .timeout_seconds
                .or(config.timeout_seconds)
                .unwrap_or(30),
            ctest: args.ctest || config_ctest.unwrap_or(false),
            test_command: args.test_command.or(config_test_command),
            clang_tidy: cli_requested_clang_tidy || config_clang_tidy.unwrap_or(false),
            clang_tidy_bin: args
                .clang_tidy_bin
                .or(config_clang_tidy_bin)
                .unwrap_or_else(|| "clang-tidy".to_string()),
            clang_tidy_checks: args.clang_tidy_checks.or(config_clang_tidy_checks),
            coverage,
            llvm_cov_bin: args
                .llvm_cov_bin
                .or(config_llvm_cov_bin)
                .unwrap_or_else(|| "llvm-cov".to_string()),
            llvm_profdata_bin: args
                .llvm_profdata_bin
                .or(config_llvm_profdata_bin)
                .unwrap_or_else(|| "llvm-profdata".to_string()),
            coverage_sources,
            coverage_objects,
            changed_lines,
            fuzz,
            fuzz_seconds,
            fuzz_corpus,
            baseline,
            max_warnings: args.max_warnings.or(config_max_warnings),
            max_analyzer_findings,
            min_line_coverage,
            min_changed_line_coverage,
            fail_on_new_diagnostics,
        })
    }
}

fn create_dir(path: &Path) -> Result<(), AppError> {
    fs::create_dir_all(path).map_err(|source| AppError::CreateDir {
        path: path.to_path_buf(),
        source,
    })
}

fn compile_stage(
    name: &str,
    compiler: &str,
    standard_flag: &str,
    source: &Path,
    output: &Path,
    extra_flags: &[String],
    timeout: Duration,
) -> Result<StageReport, AppError> {
    let mut args = vec![
        standard_flag.to_string(),
        "-Wall".to_string(),
        "-Wextra".to_string(),
        "-Wpedantic".to_string(),
        "-g".to_string(),
    ];
    args.extend(extra_flags.iter().cloned());
    args.extend([
        source.display().to_string(),
        "-o".to_string(),
        output.display().to_string(),
    ]);

    let result = run_command(CommandSpec::new(compiler).args(args), timeout)?;
    Ok(stage_from_result(name, result, Some(output)))
}

fn compilation_database_compile_stage(
    unit: &CompilationUnit,
    timeout: Duration,
) -> Result<StageReport, AppError> {
    let Some((program, args)) = unit.arguments.split_first() else {
        return Err(AppError::InvalidCompilationCommand(unit.file.clone()));
    };
    let mut args = args.to_vec();
    args.push("-fsyntax-only".to_string());

    let source = source_path(unit);
    let result = run_command(
        CommandSpec::new(program)
            .args(args)
            .current_dir(unit.directory.clone()),
        timeout,
    )?;

    Ok(stage_from_result(
        format!("compile:{}", source.display()),
        result,
        None,
    ))
}

fn compilation_database_coverage_compile_stage(
    unit: &CompilationUnit,
    output: &Path,
    timeout: Duration,
) -> Result<StageReport, AppError> {
    let Some((program, args)) = unit.arguments.split_first() else {
        return Err(AppError::InvalidCompilationCommand(unit.file.clone()));
    };

    let source = source_path(unit);
    let result = run_command(
        CommandSpec::new(program)
            .args(coverage_compilation_args(args, output))
            .current_dir(unit.directory.clone()),
        timeout,
    )?;

    Ok(stage_from_result(
        format!("coverage_compile:{}", source.display()),
        result,
        Some(output),
    ))
}

fn coverage_compilation_args(args: &[String], output: &Path) -> Vec<String> {
    let mut rewritten = Vec::new();
    let mut saw_output = false;
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        if arg == "-fsyntax-only" {
            continue;
        }

        if arg == "-o" {
            rewritten.push("-o".to_string());
            rewritten.push(output.display().to_string());
            saw_output = true;
            let _ = iter.next();
        } else if arg.starts_with("-o") && arg.len() > 2 {
            rewritten.push("-o".to_string());
            rewritten.push(output.display().to_string());
            saw_output = true;
        } else {
            rewritten.push(arg.clone());
        }
    }

    rewritten.extend(coverage_flags());
    if !saw_output {
        rewritten.push("-o".to_string());
        rewritten.push(output.display().to_string());
    }

    rewritten
}

fn fuzz_compilation_args(
    args: &[String],
    source: &Path,
    output: &Path,
    fuzz_flags: &[String],
) -> Vec<String> {
    let mut rewritten = Vec::new();
    let mut saw_source = false;
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        if arg == "-fsyntax-only" || arg == "-c" || arg == "-S" {
            continue;
        }

        if arg == "-o" {
            let _ = iter.next();
            continue;
        }

        if arg.starts_with("-o") && arg.len() > 2 {
            continue;
        }

        if source_arg_matches(arg, source) {
            saw_source = true;
        }
        rewritten.push(arg.clone());
    }

    if !saw_source {
        rewritten.push(source.display().to_string());
    }

    rewritten.extend(fuzz_flags.iter().cloned());
    rewritten.push("-o".to_string());
    rewritten.push(output.display().to_string());
    rewritten
}

fn source_arg_matches(arg: &str, source: &Path) -> bool {
    if arg.starts_with('-') {
        return false;
    }

    let path = Path::new(arg);
    path == source || source.ends_with(path)
}

fn clang_tidy_source_stage(
    clang_tidy_bin: &str,
    checks: Option<&str>,
    source: &Path,
    standard_flag: &str,
    timeout: Duration,
) -> StageReport {
    let mut args = clang_tidy_args(checks);
    args.extend([
        source.display().to_string(),
        "--".to_string(),
        standard_flag.to_string(),
    ]);

    let spec = CommandSpec::new(clang_tidy_bin).args(args);
    let command = spec.command_line();
    let result = run_command(spec, timeout);
    stage_from_command_result("clang_tidy", command, result, None)
}

fn clang_tidy_compilation_database_stage(
    unit: &CompilationUnit,
    clang_tidy_bin: &str,
    checks: Option<&str>,
    database_dir: &Path,
    timeout: Duration,
) -> StageReport {
    let source = source_path(unit);
    let mut args = clang_tidy_args(checks);
    args.extend([
        source.display().to_string(),
        "-p".to_string(),
        database_dir.display().to_string(),
    ]);

    let spec = CommandSpec::new(clang_tidy_bin)
        .args(args)
        .current_dir(unit.directory.clone());
    let command = spec.command_line();
    let result = run_command(spec, timeout);

    stage_from_command_result(
        format!("clang_tidy:{}", source.display()),
        command,
        result,
        None,
    )
}

fn clang_tidy_args(checks: Option<&str>) -> Vec<String> {
    checks
        .filter(|checks| !checks.trim().is_empty())
        .map(|checks| vec![format!("--checks={checks}")])
        .unwrap_or_default()
}

fn append_coverage_stages(
    stages: &mut Vec<StageReport>,
    args: &ResolvedCheckArgs,
    source: &Path,
    build_dir: &Path,
    artifact_root: &Path,
    timeout: Duration,
) -> Result<Option<CoverageSummary>, AppError> {
    let coverage_dir = artifact_root.join("coverage");
    create_dir(&coverage_dir)?;
    let coverage_dir = coverage_dir.canonicalize().unwrap_or(coverage_dir);

    if stages
        .iter()
        .any(|stage| stage.status == StageStatus::Failed)
    {
        append_skipped_coverage_stages(stages);
        return Ok(None);
    }

    let executable = build_dir.join(coverage_executable_name(source));
    let artifact_stem = artifact_stem(source);
    let profraw = coverage_dir.join(format!("{artifact_stem}.profraw"));
    let profdata = coverage_dir.join(format!("{artifact_stem}.profdata"));
    let summary_path = coverage_dir.join("coverage-summary.json");

    let coverage_compile = compile_stage(
        "coverage_compile",
        &args.compiler,
        args.standard.as_flag(),
        source,
        &executable,
        &coverage_flags(),
        timeout,
    )?;
    let coverage_compile_ok = coverage_compile.status == StageStatus::Passed;
    stages.push(coverage_compile);
    if !coverage_compile_ok {
        stages.push(StageReport::skipped("coverage_run"));
        stages.push(StageReport::skipped("coverage_merge"));
        stages.push(StageReport::skipped("coverage_report"));
        return Ok(None);
    }

    let coverage_run = coverage_run_stage(&executable, &profraw, timeout);
    let coverage_run_ok = coverage_run.status == StageStatus::Passed;
    stages.push(coverage_run);
    if !coverage_run_ok {
        stages.push(StageReport::skipped("coverage_merge"));
        stages.push(StageReport::skipped("coverage_report"));
        return Ok(None);
    }

    let coverage_merge = coverage_merge_stage(
        "coverage_merge",
        &args.llvm_profdata_bin,
        &[profraw],
        &profdata,
        timeout,
    );
    let coverage_merge_ok = coverage_merge.status == StageStatus::Passed;
    stages.push(coverage_merge);
    if !coverage_merge_ok {
        stages.push(StageReport::skipped("coverage_report"));
        return Ok(None);
    }

    let (coverage_report, coverage) = coverage_report_stage(
        &args.llvm_cov_bin,
        &selected_coverage_objects(args, vec![executable]),
        &profdata,
        &selected_coverage_sources(args, vec![source.to_path_buf()]),
        &args.changed_lines,
        &summary_path,
        timeout,
    )?;
    stages.push(coverage_report);

    Ok(coverage)
}

fn append_compilation_database_coverage_stages(
    stages: &mut Vec<StageReport>,
    args: &ResolvedCheckArgs,
    database: &CompilationDatabase,
    artifact_root: &Path,
    timeout: Duration,
) -> Result<Option<CoverageSummary>, AppError> {
    let coverage_dir = artifact_root.join("coverage").join("compilation-database");
    let objects_dir = coverage_dir.join("objects");
    create_dir(&objects_dir)?;
    let coverage_dir = coverage_dir.canonicalize().unwrap_or(coverage_dir);
    let objects_dir = objects_dir.canonicalize().unwrap_or(objects_dir);

    if stages
        .iter()
        .any(|stage| stage.status == StageStatus::Failed)
    {
        append_skipped_compilation_database_coverage_stages(stages, database);
        return Ok(None);
    }

    let mut objects = Vec::new();
    for (index, unit) in database.entries.iter().enumerate() {
        let source = source_path(unit);
        let object = objects_dir.join(format!("{:04}-{}.o", index + 1, artifact_stem(&source)));
        let coverage_compile = compilation_database_coverage_compile_stage(unit, &object, timeout)?;
        let coverage_compile_ok = coverage_compile.status == StageStatus::Passed;
        stages.push(coverage_compile);
        if coverage_compile_ok {
            objects.push(object);
        }
    }

    if objects.len() != database.entries.len() {
        stages.push(StageReport::skipped("coverage_test_command"));
        stages.push(StageReport::skipped("coverage_merge"));
        stages.push(StageReport::skipped("coverage_report"));
        return Ok(None);
    }

    let Some(test_command) = args.test_command.as_deref() else {
        stages.push(failed_stage(
            "coverage_test_command",
            Vec::new(),
            "raw compile_commands.json coverage requires --test-command to run coverage-instrumented tests",
            Some(&coverage_dir),
        ));
        stages.push(StageReport::skipped("coverage_merge"));
        stages.push(StageReport::skipped("coverage_report"));
        return Ok(None);
    };

    let profraw_pattern = coverage_dir.join("compdb-%p.profraw");
    let database_dir = database.path.parent();
    let coverage_test_command =
        coverage_test_command_stage(test_command, database_dir, &profraw_pattern, timeout);
    let coverage_test_command_ok = coverage_test_command.status == StageStatus::Passed;
    stages.push(coverage_test_command);
    if !coverage_test_command_ok {
        stages.push(StageReport::skipped("coverage_merge"));
        stages.push(StageReport::skipped("coverage_report"));
        return Ok(None);
    }

    let profraws = collect_files_with_extension(&coverage_dir, "profraw");
    let profdata = coverage_dir.join("compilation-database.profdata");
    if profraws.is_empty() {
        stages.push(missing_coverage_input_stage(
            "coverage_merge",
            &args.llvm_profdata_bin,
            &coverage_dir,
            &profdata,
            "no coverage profile files were produced by the test command",
        ));
        stages.push(StageReport::skipped("coverage_report"));
        return Ok(None);
    }

    let coverage_merge = coverage_merge_stage(
        "coverage_merge",
        &args.llvm_profdata_bin,
        &profraws,
        &profdata,
        timeout,
    );
    let coverage_merge_ok = coverage_merge.status == StageStatus::Passed;
    stages.push(coverage_merge);
    if !coverage_merge_ok {
        stages.push(StageReport::skipped("coverage_report"));
        return Ok(None);
    }

    let summary_path = coverage_dir.join("coverage-summary.json");
    let sources = database.entries.iter().map(source_path).collect::<Vec<_>>();
    let (coverage_report, coverage) = coverage_report_stage(
        &args.llvm_cov_bin,
        &selected_coverage_objects(args, objects),
        &profdata,
        &selected_coverage_sources(args, sources),
        &args.changed_lines,
        &summary_path,
        timeout,
    )?;
    stages.push(coverage_report);

    Ok(coverage)
}

fn append_skipped_coverage_stages(stages: &mut Vec<StageReport>) {
    stages.push(StageReport::skipped("coverage_compile"));
    stages.push(StageReport::skipped("coverage_run"));
    stages.push(StageReport::skipped("coverage_merge"));
    stages.push(StageReport::skipped("coverage_report"));
}

fn append_skipped_compilation_database_coverage_stages(
    stages: &mut Vec<StageReport>,
    database: &CompilationDatabase,
) {
    stages.extend(database.entries.iter().map(|unit| {
        StageReport::skipped(format!("coverage_compile:{}", source_path(unit).display()))
    }));
    stages.push(StageReport::skipped("coverage_test_command"));
    stages.push(StageReport::skipped("coverage_merge"));
    stages.push(StageReport::skipped("coverage_report"));
}

fn append_cmake_coverage_stages(
    stages: &mut Vec<StageReport>,
    args: &ResolvedCheckArgs,
    project_dir: &Path,
    artifact_root: &Path,
    timeout: Duration,
) -> Result<Option<CoverageSummary>, AppError> {
    let coverage_build_dir = artifact_root.join("cmake-coverage-build");
    let coverage_dir = artifact_root.join("coverage").join("cmake");
    create_dir(&coverage_dir)?;
    let coverage_dir = coverage_dir.canonicalize().unwrap_or(coverage_dir);

    if stages
        .iter()
        .any(|stage| stage.status == StageStatus::Failed)
    {
        append_skipped_cmake_coverage_stages(stages);
        return Ok(None);
    }

    let coverage_configure =
        cmake_coverage_configure_stage(project_dir, &coverage_build_dir, &args.compiler, timeout);
    let coverage_configure_ok = coverage_configure.status == StageStatus::Passed;
    stages.push(coverage_configure);
    if !coverage_configure_ok {
        stages.push(StageReport::skipped("coverage_cmake_build"));
        stages.push(StageReport::skipped("coverage_ctest"));
        stages.push(StageReport::skipped("coverage_merge"));
        stages.push(StageReport::skipped("coverage_report"));
        return Ok(None);
    }

    let coverage_build =
        cmake_build_stage_named("coverage_cmake_build", &coverage_build_dir, timeout);
    let coverage_build_ok = coverage_build.status == StageStatus::Passed;
    stages.push(coverage_build);
    if !coverage_build_ok {
        stages.push(StageReport::skipped("coverage_ctest"));
        stages.push(StageReport::skipped("coverage_merge"));
        stages.push(StageReport::skipped("coverage_report"));
        return Ok(None);
    }

    let profraw_pattern = coverage_dir.join("cmake-%p.profraw");
    let coverage_ctest = ctest_stage_named(
        "coverage_ctest",
        &coverage_build_dir,
        Some(&profraw_pattern),
        timeout,
    );
    let coverage_ctest_ok = coverage_ctest.status == StageStatus::Passed;
    stages.push(coverage_ctest);
    if !coverage_ctest_ok {
        stages.push(StageReport::skipped("coverage_merge"));
        stages.push(StageReport::skipped("coverage_report"));
        return Ok(None);
    }

    let profraws = collect_files_with_extension(&coverage_dir, "profraw");
    let profdata = coverage_dir.join("cmake.profdata");
    if profraws.is_empty() {
        stages.push(missing_coverage_input_stage(
            "coverage_merge",
            &args.llvm_profdata_bin,
            &coverage_dir,
            &profdata,
            "no coverage profile files were produced by CTest",
        ));
        stages.push(StageReport::skipped("coverage_report"));
        return Ok(None);
    }

    let coverage_merge = coverage_merge_stage(
        "coverage_merge",
        &args.llvm_profdata_bin,
        &profraws,
        &profdata,
        timeout,
    );
    let coverage_merge_ok = coverage_merge.status == StageStatus::Passed;
    stages.push(coverage_merge);
    if !coverage_merge_ok {
        stages.push(StageReport::skipped("coverage_report"));
        return Ok(None);
    }

    let summary_path = coverage_dir.join("coverage-summary.json");
    let objects = discover_coverage_objects(&coverage_build_dir);
    let (coverage_report, coverage) = coverage_report_stage(
        &args.llvm_cov_bin,
        &selected_coverage_objects(args, objects),
        &profdata,
        &selected_coverage_sources(args, Vec::new()),
        &args.changed_lines,
        &summary_path,
        timeout,
    )?;
    stages.push(coverage_report);

    Ok(coverage)
}

fn selected_coverage_sources(
    args: &ResolvedCheckArgs,
    default_sources: Vec<PathBuf>,
) -> Vec<PathBuf> {
    if args.coverage_sources.is_empty() {
        default_sources
    } else {
        args.coverage_sources.clone()
    }
}

fn selected_coverage_objects(
    args: &ResolvedCheckArgs,
    default_objects: Vec<PathBuf>,
) -> Vec<PathBuf> {
    if args.coverage_objects.is_empty() {
        default_objects
    } else {
        args.coverage_objects.clone()
    }
}

fn append_skipped_cmake_coverage_stages(stages: &mut Vec<StageReport>) {
    stages.push(StageReport::skipped("coverage_cmake_configure"));
    stages.push(StageReport::skipped("coverage_cmake_build"));
    stages.push(StageReport::skipped("coverage_ctest"));
    stages.push(StageReport::skipped("coverage_merge"));
    stages.push(StageReport::skipped("coverage_report"));
}

fn coverage_run_stage(executable: &Path, profraw: &Path, timeout: Duration) -> StageReport {
    let spec = CommandSpec::new(executable.display().to_string())
        .env("LLVM_PROFILE_FILE", profraw.display().to_string());
    let command = spec.command_line();
    let result = run_command(spec, timeout);
    stage_from_command_result("coverage_run", command, result, Some(profraw))
}

fn coverage_test_command_stage(
    command: &str,
    current_dir: Option<&Path>,
    profraw_pattern: &Path,
    timeout: Duration,
) -> StageReport {
    let mut spec =
        shell_command_spec(command).env("LLVM_PROFILE_FILE", profraw_pattern.display().to_string());
    if let Some(current_dir) = current_dir {
        spec = spec.current_dir(current_dir.to_path_buf());
    }

    let command_line = spec.command_line();
    let result = run_command(spec, timeout);
    stage_from_command_result(
        "coverage_test_command",
        command_line,
        result,
        Some(profraw_pattern),
    )
}

fn coverage_merge_stage(
    name: &str,
    llvm_profdata_bin: &str,
    profraws: &[PathBuf],
    profdata: &Path,
    timeout: Duration,
) -> StageReport {
    let mut args = vec!["merge".to_string(), "-sparse".to_string()];
    args.extend(profraws.iter().map(|path| path.display().to_string()));
    args.extend(["-o".to_string(), profdata.display().to_string()]);
    let spec = CommandSpec::new(llvm_profdata_bin).args(args);
    let command = spec.command_line();
    let result = run_command(spec, timeout);
    stage_from_command_result(name, command, result, Some(profdata))
}

fn coverage_report_stage(
    llvm_cov_bin: &str,
    objects: &[PathBuf],
    profdata: &Path,
    sources: &[PathBuf],
    changed_lines: &[ChangedLine],
    summary_path: &Path,
    timeout: Duration,
) -> Result<(StageReport, Option<CoverageSummary>), AppError> {
    if objects.is_empty() {
        return Ok((
            failed_stage(
                "coverage_report",
                vec![
                    llvm_cov_bin.to_string(),
                    "export".to_string(),
                    "<coverage-object>".to_string(),
                    format!("-instr-profile={}", profdata.display()),
                ],
                "no coverage object files were found",
                Some(summary_path),
            ),
            None,
        ));
    }

    let mut args = vec![
        "export".to_string(),
        objects[0].display().to_string(),
        format!("-instr-profile={}", profdata.display()),
    ];
    if changed_lines.is_empty() {
        args.push("--summary-only".to_string());
    }
    args.extend(
        objects
            .iter()
            .skip(1)
            .map(|object| format!("--object={}", object.display())),
    );
    for source in sources {
        args.extend(["--sources".to_string(), source.display().to_string()]);
    }
    let spec = CommandSpec::new(llvm_cov_bin).args(args);
    let command = spec.command_line();
    let result = run_command(spec, timeout);
    let mut stage =
        stage_from_command_result("coverage_report", command, result, Some(summary_path));

    if stage.status != StageStatus::Passed {
        return Ok((stage, None));
    }

    let coverage = match parse_coverage_summary(&stage.stdout, changed_lines) {
        Ok(coverage) => coverage,
        Err(error) => {
            stage.status = StageStatus::Failed;
            stage.errors += 1;
            if !stage.stderr.is_empty() {
                stage.stderr.push('\n');
            }
            stage.stderr.push_str(&error);
            return Ok((stage, None));
        }
    };

    if let Some(parent) = summary_path.parent() {
        create_dir(parent)?;
    }
    fs::write(summary_path, &stage.stdout).map_err(|source| AppError::WriteReport {
        path: summary_path.to_path_buf(),
        source,
    })?;

    Ok((stage, Some(coverage)))
}

fn missing_coverage_input_stage(
    name: &str,
    llvm_profdata_bin: &str,
    coverage_dir: &Path,
    profdata: &Path,
    message: &str,
) -> StageReport {
    failed_stage(
        name,
        vec![
            llvm_profdata_bin.to_string(),
            "merge".to_string(),
            "-sparse".to_string(),
            format!("{}/*.profraw", coverage_dir.display()),
            "-o".to_string(),
            profdata.display().to_string(),
        ],
        message,
        Some(profdata),
    )
}

fn failed_stage(
    name: impl Into<String>,
    command: Vec<String>,
    message: &str,
    artifact: Option<&Path>,
) -> StageReport {
    StageReport {
        name: name.into(),
        status: StageStatus::Failed,
        command,
        exit_code: None,
        timed_out: false,
        warnings: 0,
        errors: 1,
        diagnostics: Vec::new(),
        stdout: String::new(),
        stderr: message.to_string(),
        artifact: artifact.map(Path::to_path_buf),
    }
}

fn collect_files_with_extension(root: &Path, extension: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_files(root, &mut paths, &|path| {
        path.extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value == extension)
    });
    paths.sort();
    paths
}

fn discover_coverage_objects(build_dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_files(build_dir, &mut paths, &is_coverage_object);
    paths.sort();
    paths
}

fn collect_files(root: &Path, paths: &mut Vec<PathBuf>, matches: &dyn Fn(&Path) -> bool) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, paths, matches);
        } else if matches(&path) {
            paths.push(path);
        }
    }
}

fn is_coverage_object(path: &Path) -> bool {
    if path
        .components()
        .any(|component| component.as_os_str().to_string_lossy() == "CMakeFiles")
    {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        path.is_file()
            && fs::metadata(path)
                .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

fn parse_coverage_summary(
    output: &str,
    changed_lines: &[ChangedLine],
) -> Result<CoverageSummary, String> {
    let value: serde_json::Value = serde_json::from_str(output)
        .map_err(|source| format!("failed to parse coverage JSON: {source}"))?;
    let totals = value
        .pointer("/data/0/totals")
        .ok_or_else(|| "coverage JSON is missing data[0].totals".to_string())?;

    Ok(CoverageSummary {
        lines: coverage_metric(totals, "lines")?,
        functions: coverage_metric(totals, "functions")?,
        regions: coverage_metric(totals, "regions")?,
        changed_lines: changed_line_coverage_metric(&value, changed_lines)?,
    })
}

fn changed_line_coverage_metric(
    value: &serde_json::Value,
    changed_lines: &[ChangedLine],
) -> Result<Option<CoverageMetric>, String> {
    if changed_lines.is_empty() {
        return Ok(None);
    }

    let files = value
        .pointer("/data/0/files")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            "coverage JSON is missing data[0].files; changed-line coverage requires non-summary llvm-cov output".to_string()
        })?;

    let mut covered = 0;
    let mut count = 0;

    for changed_line in changed_lines {
        if let Some(is_covered) = changed_line_is_covered(files, changed_line) {
            count += 1;
            if is_covered {
                covered += 1;
            }
        }
    }

    let percent = if count == 0 {
        100.0
    } else {
        (covered as f64 / count as f64) * 100.0
    };

    Ok(Some(CoverageMetric {
        count,
        covered,
        percent,
    }))
}

fn changed_line_is_covered(
    files: &[serde_json::Value],
    changed_line: &ChangedLine,
) -> Option<bool> {
    files.iter().find_map(|file| {
        let filename = file.get("filename")?.as_str()?;
        if !coverage_path_matches(filename, &changed_line.path) {
            return None;
        }

        let line_counts = coverage_line_counts(file);
        line_counts.get(&changed_line.line).map(|count| *count > 0)
    })
}

fn coverage_line_counts(file: &serde_json::Value) -> HashMap<u64, u64> {
    let mut counts = HashMap::new();
    let Some(segments) = file.get("segments").and_then(serde_json::Value::as_array) else {
        return counts;
    };

    for segment in segments {
        let Some(values) = segment.as_array() else {
            continue;
        };
        let Some(line) = values.first().and_then(serde_json::Value::as_u64) else {
            continue;
        };
        let Some(count) = values.get(2).and_then(serde_json::Value::as_u64) else {
            continue;
        };
        let has_count = values
            .get(3)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        if has_count {
            counts
                .entry(line)
                .and_modify(|current| *current = (*current).max(count))
                .or_insert(count);
        }
    }

    counts
}

fn coverage_path_matches(filename: &str, changed_path: &Path) -> bool {
    let coverage_path = Path::new(filename);
    coverage_path == changed_path
        || coverage_path.ends_with(changed_path)
        || changed_path.ends_with(coverage_path)
}

fn coverage_metric(totals: &serde_json::Value, key: &str) -> Result<CoverageMetric, String> {
    let metric = totals
        .get(key)
        .ok_or_else(|| format!("coverage JSON is missing totals.{key}"))?;
    let count = metric
        .get("count")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("coverage JSON is missing totals.{key}.count"))?;
    let covered = metric
        .get("covered")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("coverage JSON is missing totals.{key}.covered"))?;
    let percent = metric
        .get("percent")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| format!("coverage JSON is missing totals.{key}.percent"))?;

    Ok(CoverageMetric {
        count,
        covered,
        percent,
    })
}

fn coverage_flags() -> Vec<String> {
    vec![
        "-fprofile-instr-generate".to_string(),
        "-fcoverage-mapping".to_string(),
    ]
}

fn run_executable_stage(
    name: &str,
    executable: &Path,
    timeout: Duration,
) -> Result<StageReport, AppError> {
    let result = run_command(CommandSpec::new(executable.display().to_string()), timeout)?;
    Ok(stage_from_result(name, result, Some(executable)))
}

fn is_compilation_database_file(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "compile_commands.json")
}

fn is_cmake_project(path: &Path) -> bool {
    path.join("CMakeLists.txt").is_file()
}

fn source_path(unit: &CompilationUnit) -> PathBuf {
    if unit.file.is_absolute() {
        unit.file.clone()
    } else {
        unit.directory.join(&unit.file)
    }
}

fn parse_sanitizers(value: &str) -> Result<Vec<Sanitizer>, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
        return Ok(Vec::new());
    }

    let mut sanitizers = Vec::new();
    for item in trimmed
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let sanitizer = match item {
            "address" | "asan" => Sanitizer::Address,
            "undefined" | "ubsan" => Sanitizer::Undefined,
            invalid => return Err(AppError::InvalidSanitizer(invalid.to_string())),
        };

        if !sanitizers.contains(&sanitizer) {
            sanitizers.push(sanitizer);
        }
    }

    Ok(sanitizers)
}

fn sanitizer_flags(sanitizers: &[Sanitizer]) -> Vec<String> {
    let joined = sanitizers
        .iter()
        .map(|sanitizer| sanitizer.compiler_name())
        .collect::<Vec<_>>()
        .join(",");

    vec![
        "-O1".to_string(),
        "-fno-omit-frame-pointer".to_string(),
        "-fno-sanitize-recover=all".to_string(),
        format!("-fsanitize={joined}"),
    ]
}

fn fuzz_flags(sanitizers: &[Sanitizer]) -> Vec<String> {
    let mut sanitizer_names = vec!["fuzzer"];
    sanitizer_names.extend(sanitizers.iter().map(|sanitizer| sanitizer.compiler_name()));

    vec![
        "-O1".to_string(),
        "-fno-omit-frame-pointer".to_string(),
        "-fno-sanitize-recover=all".to_string(),
        format!("-fsanitize={}", sanitizer_names.join(",")),
    ]
}

fn summarize(
    stages: &[StageReport],
    coverage: Option<CoverageSummary>,
    baseline: Option<BaselineSummary>,
) -> Summary {
    Summary {
        warnings: stages.iter().map(|stage| stage.warnings).sum(),
        errors: stages.iter().map(|stage| stage.errors).sum(),
        diagnostics: stages.iter().map(|stage| stage.diagnostics.len()).sum(),
        failed_stages: stages
            .iter()
            .filter(|stage| stage.status == StageStatus::Failed)
            .count(),
        timed_out_stages: stages.iter().filter(|stage| stage.timed_out).count(),
        coverage,
        baseline,
    }
}

fn write_report(report: &Report) -> Result<(), AppError> {
    if let Some(parent) = report.report_path.parent() {
        create_dir(parent)?;
    }

    let serialized = serde_json::to_string_pretty(report)?;
    fs::write(&report.report_path, serialized).map_err(|source| AppError::WriteReport {
        path: report.report_path.clone(),
        source,
    })
}

fn write_report_outputs(report: &Report) -> Result<(), AppError> {
    write_report(report)?;

    if let Some(path) = &report.markdown_report_path {
        if let Some(parent) = path.parent() {
            create_dir(parent)?;
        }
        fs::write(path, report.render_markdown()).map_err(|source| AppError::WriteReport {
            path: path.clone(),
            source,
        })?;
    }

    if let Some(path) = &report.html_report_path {
        if let Some(parent) = path.parent() {
            create_dir(parent)?;
        }
        fs::write(path, report.render_html()).map_err(|source| AppError::WriteReport {
            path: path.clone(),
            source,
        })?;
    }

    if let Some(path) = &report.sarif_report_path {
        if let Some(parent) = path.parent() {
            create_dir(parent)?;
        }
        fs::write(path, report.render_sarif()).map_err(|source| AppError::WriteReport {
            path: path.clone(),
            source,
        })?;
    }

    Ok(())
}

fn executable_name(source: &Path, sanitizers: Option<&[Sanitizer]>) -> String {
    let base = artifact_stem(source);

    let suffix = sanitizers
        .map(|items| {
            items
                .iter()
                .map(|sanitizer| sanitizer.artifact_name())
                .collect::<Vec<_>>()
                .join("-")
        })
        .filter(|suffix| !suffix.is_empty());

    match suffix {
        Some(suffix) => format!("{base}-{suffix}{}", std::env::consts::EXE_SUFFIX),
        None => format!("{base}{}", std::env::consts::EXE_SUFFIX),
    }
}

fn coverage_executable_name(source: &Path) -> String {
    format!(
        "{}-coverage{}",
        artifact_stem(source),
        std::env::consts::EXE_SUFFIX
    )
}

fn fuzz_executable_name(source: &Path) -> String {
    format!(
        "{}-fuzz{}",
        artifact_stem(source),
        std::env::consts::EXE_SUFFIX
    )
}

fn project_fuzz_executable_name(target: &FuzzTarget) -> String {
    format!("{}{}", target.artifact_id, std::env::consts::EXE_SUFFIX)
}

fn fuzz_stage_name(kind: &str, source: &Path) -> String {
    format!("{kind}:{}", source.display())
}

fn artifact_stem(source: &Path) -> String {
    source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(sanitize_filename)
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "cppgauntlet-target".to_string())
}

fn sanitize_path_id(path: &Path) -> String {
    let value = sanitize_filename(&path.display().to_string());
    let trimmed = value.trim_matches('_');
    if trimmed.is_empty() {
        "cppgauntlet-target".to_string()
    } else {
        trimmed.to_string()
    }
}

fn sanitize_filename(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
