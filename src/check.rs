use std::fs;
use std::path::Path;
use std::time::Duration;

use crate::cli::CheckArgs;
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
    validate_target(&args.file)?;

    let source = args.file.canonicalize().unwrap_or(args.file.clone());
    let artifact_root = args.artifact_dir;
    let build_dir = artifact_root.join("build");
    create_dir(&artifact_root)?;
    create_dir(&build_dir)?;

    let report_path = args
        .report
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

fn validate_target(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Err(AppError::TargetMissing(path.to_path_buf()));
    }

    if !path.is_file() {
        return Err(AppError::TargetNotFile(path.to_path_buf()));
    }

    Ok(())
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

fn run_executable_stage(
    name: &str,
    executable: &Path,
    timeout: Duration,
) -> Result<StageReport, AppError> {
    let result = run_command(CommandSpec::new(executable.display().to_string()), timeout)?;
    Ok(stage_from_result(name, result, Some(executable)))
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
