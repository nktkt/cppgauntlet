use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::cli::{CheckArgs, CppStandard};
use crate::compdb::{self, CompilationDatabase, CompilationUnit};
use crate::config;
use crate::error::AppError;
use crate::report::{
    stage_from_result, Report, ReportStatus, StageReport, StageStatus, Summary, TargetInfo,
    ToolInfo,
};
use crate::runner::{run_command, CommandSpec};

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
    let target = CheckTarget::resolve(&args.file)?;

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

    let report_path = args
        .report
        .clone()
        .unwrap_or_else(|| artifact_root.join("cppgauntlet-report.json"));
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

    let summary = summarize(&stages);
    let status = if summary.failed_stages == 0 {
        ReportStatus::Passed
    } else {
        ReportStatus::Failed
    };

    let report = Report {
        schema_version: 2,
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
        report_path,
    };

    write_report(&report)?;
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

    let report_path = args
        .report
        .clone()
        .unwrap_or_else(|| artifact_root.join("cppgauntlet-report.json"));
    let timeout = Duration::from_secs(args.timeout_seconds);
    append_compilation_database_compile_stages(&mut stages, &database, timeout)?;
    append_clang_tidy_stages(&mut stages, &database, &args, timeout);

    build_and_write_report(
        database.path,
        "from compilation database".to_string(),
        "from compilation database".to_string(),
        stages,
        report_path,
    )
}

fn run_cmake_project(args: ResolvedCheckArgs, project_dir: PathBuf) -> Result<Report, AppError> {
    let artifact_root = args.artifact_dir.clone();
    let build_dir = artifact_root.join("cmake-build");
    create_dir(&artifact_root)?;

    let report_path = args
        .report
        .clone()
        .unwrap_or_else(|| artifact_root.join("cppgauntlet-report.json"));
    let timeout = Duration::from_secs(args.timeout_seconds);
    let cmake_stage = cmake_configure_stage(&project_dir, &build_dir, timeout);

    if cmake_stage.status == StageStatus::Failed {
        return build_and_write_report(
            project_dir,
            "from CMake".to_string(),
            "from CMake".to_string(),
            vec![cmake_stage],
            report_path,
        );
    }

    let database = compdb::load_for_target(&build_dir)?;
    let mut stages = vec![cmake_stage];
    let timeout = Duration::from_secs(args.timeout_seconds);
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

    build_and_write_report(
        project_dir,
        "from CMake".to_string(),
        "from CMake".to_string(),
        stages,
        report_path,
    )
}

fn build_and_write_report(
    target_path: PathBuf,
    standard: String,
    compiler: String,
    stages: Vec<StageReport>,
    report_path: PathBuf,
) -> Result<Report, AppError> {
    let summary = summarize(&stages);
    let status = if summary.failed_stages == 0 {
        ReportStatus::Passed
    } else {
        ReportStatus::Failed
    };

    let report = Report {
        schema_version: 2,
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
        report_path,
    };

    write_report(&report)?;
    Ok(report)
}

fn cmake_configure_stage(source_dir: &Path, build_dir: &Path, timeout: Duration) -> StageReport {
    let args = vec![
        "-S".to_string(),
        source_dir.display().to_string(),
        "-B".to_string(),
        build_dir.display().to_string(),
        "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON".to_string(),
    ];
    let spec = CommandSpec::new("cmake").args(args);
    let command = spec.command_line();

    match run_command(spec, timeout) {
        Ok(result) => stage_from_result("cmake_configure", result, Some(build_dir)),
        Err(error) => StageReport {
            name: "cmake_configure".to_string(),
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
    let args = vec!["--build".to_string(), build_dir.display().to_string()];
    let spec = CommandSpec::new("cmake").args(args);
    let command = spec.command_line();
    let result = run_command(spec, timeout);
    stage_from_command_result("cmake_build", command, result, Some(build_dir))
}

fn ctest_stage(build_dir: &Path, timeout: Duration) -> StageReport {
    let args = vec![
        "--test-dir".to_string(),
        build_dir.display().to_string(),
        "--output-on-failure".to_string(),
    ];
    let spec = CommandSpec::new("ctest").args(args);
    let command = spec.command_line();
    let result = run_command(spec, timeout);
    stage_from_command_result("ctest", command, result, Some(build_dir))
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

#[derive(Debug)]
struct ResolvedCheckArgs {
    file: PathBuf,
    standard: CppStandard,
    compiler: String,
    sanitizers: String,
    artifact_dir: PathBuf,
    report: Option<PathBuf>,
    timeout_seconds: u64,
    ctest: bool,
    clang_tidy: bool,
    clang_tidy_bin: String,
    clang_tidy_checks: Option<String>,
}

impl ResolvedCheckArgs {
    fn from_cli(args: CheckArgs) -> Result<Self, AppError> {
        let config = config::load_config(args.config.as_deref())?;
        let config_standard = config.standard()?;
        let config_sanitizers = config.sanitizers_csv();
        let config_report = config.report_path();
        let config_ctest = config.ctest_enabled();
        let config_clang_tidy = config.clang_tidy_enabled();
        let config_clang_tidy_bin = config.clang_tidy_bin();
        let config_clang_tidy_checks = config.clang_tidy_checks();
        let cli_requested_clang_tidy =
            args.clang_tidy || args.clang_tidy_bin.is_some() || args.clang_tidy_checks.is_some();

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
            timeout_seconds: args
                .timeout_seconds
                .or(config.timeout_seconds)
                .unwrap_or(30),
            ctest: args.ctest || config_ctest.unwrap_or(false),
            clang_tidy: cli_requested_clang_tidy || config_clang_tidy.unwrap_or(false),
            clang_tidy_bin: args
                .clang_tidy_bin
                .or(config_clang_tidy_bin)
                .unwrap_or_else(|| "clang-tidy".to_string()),
            clang_tidy_checks: args.clang_tidy_checks.or(config_clang_tidy_checks),
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

fn summarize(stages: &[StageReport]) -> Summary {
    Summary {
        warnings: stages.iter().map(|stage| stage.warnings).sum(),
        errors: stages.iter().map(|stage| stage.errors).sum(),
        diagnostics: stages.iter().map(|stage| stage.diagnostics.len()).sum(),
        failed_stages: stages
            .iter()
            .filter(|stage| stage.status == StageStatus::Failed)
            .count(),
        timed_out_stages: stages.iter().filter(|stage| stage.timed_out).count(),
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

fn executable_name(source: &Path, sanitizers: Option<&[Sanitizer]>) -> String {
    let base = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(sanitize_filename)
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "cppgauntlet-target".to_string());

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
