use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "cppgauntlet")]
#[command(
    author,
    version,
    about = "Put C++ code through a practical verification gauntlet."
)]
pub struct Cli {
    #[arg(long, value_enum, default_value_t = OutputFormat::Text, global = true)]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Markdown,
    Html,
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Commands {
    /// Compile and run checks for a single C++ source file.
    Check(CheckArgs),

    /// Manage diagnostic baseline reports.
    Baseline(BaselineArgs),

    /// Inspect external tools needed by CppGauntlet workflows.
    Doctor(DoctorArgs),

    /// Write a starter cppgauntlet.yaml configuration file.
    Init(InitArgs),
}

#[derive(Debug, Args)]
pub struct BaselineArgs {
    #[command(subcommand)]
    pub command: BaselineCommands,
}

#[derive(Debug, Subcommand)]
pub enum BaselineCommands {
    /// Export a check report as a diagnostic baseline.
    Update(BaselineUpdateArgs),
}

#[derive(Debug, Args)]
pub struct BaselineUpdateArgs {
    /// CppGauntlet JSON report to export as a baseline.
    #[arg(long, default_value = ".cppgauntlet/cppgauntlet-report.json")]
    pub report: PathBuf,

    /// Baseline JSON report path to write.
    #[arg(long, default_value = ".cppgauntlet/baseline.json")]
    pub output: PathBuf,
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    /// C++ source file, project directory, or compile_commands.json file to check.
    pub file: PathBuf,

    /// Configuration file to load. Defaults to cppgauntlet.yaml when it exists.
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// C++ language standard to pass to the compiler.
    #[arg(long, value_parser = parse_standard)]
    pub standard: Option<CppStandard>,

    /// C++ compiler executable.
    #[arg(long)]
    pub compiler: Option<String>,

    /// Comma-separated sanitizer list: address, undefined, asan, ubsan, or none.
    #[arg(long)]
    pub sanitizers: Option<String>,

    /// Root directory for generated artifacts and reports.
    #[arg(long)]
    pub artifact_dir: Option<PathBuf>,

    /// Optional report path. Defaults to <artifact-dir>/cppgauntlet-report.json.
    #[arg(long)]
    pub report: Option<PathBuf>,

    /// Optional Markdown report path.
    #[arg(long)]
    pub markdown_report: Option<PathBuf>,

    /// Optional HTML report path.
    #[arg(long)]
    pub html_report: Option<PathBuf>,

    /// Optional SARIF report path.
    #[arg(long)]
    pub sarif_report: Option<PathBuf>,

    /// Per-command timeout in seconds.
    #[arg(long)]
    pub timeout_seconds: Option<u64>,

    /// For CMake projects, build the project and run ctest after compile checks.
    #[arg(long)]
    pub ctest: bool,

    /// Run a custom test command after compile and analyzer checks.
    #[arg(long)]
    pub test_command: Option<String>,

    /// Run clang-tidy analysis after compile checks.
    #[arg(long)]
    pub clang_tidy: bool,

    /// clang-tidy executable.
    #[arg(long)]
    pub clang_tidy_bin: Option<String>,

    /// clang-tidy checks expression.
    #[arg(long)]
    pub clang_tidy_checks: Option<String>,

    /// Collect source-based coverage for source, compile database, and CMake checks.
    #[arg(long)]
    pub coverage: bool,

    /// llvm-cov executable.
    #[arg(long)]
    pub llvm_cov_bin: Option<String>,

    /// llvm-profdata executable.
    #[arg(long)]
    pub llvm_profdata_bin: Option<String>,

    /// Source path to include in llvm-cov export. Repeat to limit coverage output.
    #[arg(long = "coverage-source")]
    pub coverage_sources: Vec<PathBuf>,

    /// Coverage object/executable to pass to llvm-cov export. Repeat to override discovery.
    #[arg(long = "coverage-object")]
    pub coverage_objects: Vec<PathBuf>,

    /// Changed source line in <path>:<line> form. Repeat for changed-line coverage.
    #[arg(long = "changed-line")]
    pub changed_lines: Vec<String>,

    /// Previous CppGauntlet JSON report to use as a diagnostic baseline.
    #[arg(long)]
    pub baseline: Option<PathBuf>,

    /// Fail the report when total warnings exceed this number.
    #[arg(long)]
    pub max_warnings: Option<usize>,

    /// Fail the report when analyzer findings exceed this number.
    #[arg(long)]
    pub max_analyzer_findings: Option<usize>,

    /// Fail the report when line coverage is below this percentage.
    #[arg(long, value_parser = parse_percentage)]
    pub min_line_coverage: Option<f64>,

    /// Fail the report when changed-line coverage is below this percentage.
    #[arg(long, value_parser = parse_percentage)]
    pub min_changed_line_coverage: Option<f64>,

    /// Fail the report when diagnostics are not present in the baseline report.
    #[arg(long)]
    pub fail_on_new_diagnostics: bool,
}

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Configuration file path to create.
    #[arg(long, default_value = "cppgauntlet.yaml")]
    pub path: PathBuf,

    /// Overwrite an existing configuration file.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Required tool to check. Repeat to replace the default required tool set.
    #[arg(long = "required-tool")]
    pub required_tools: Vec<String>,

    /// Optional tool to check. Repeat to replace the default optional tool set.
    #[arg(long = "optional-tool")]
    pub optional_tools: Vec<String>,

    /// Per-tool timeout in seconds.
    #[arg(long, default_value_t = 5)]
    pub timeout_seconds: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CppStandard {
    Cxx17,
    Cxx20,
    Cxx23,
}

impl CppStandard {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "c++17" | "17" => Ok(Self::Cxx17),
            "c++20" | "20" => Ok(Self::Cxx20),
            "c++23" | "23" => Ok(Self::Cxx23),
            _ => Err(format!(
                "unsupported C++ standard '{value}'. Expected c++17, c++20, or c++23"
            )),
        }
    }

    pub fn as_flag(self) -> &'static str {
        match self {
            Self::Cxx17 => "-std=c++17",
            Self::Cxx20 => "-std=c++20",
            Self::Cxx23 => "-std=c++23",
        }
    }

    pub fn as_report_value(self) -> &'static str {
        match self {
            Self::Cxx17 => "c++17",
            Self::Cxx20 => "c++20",
            Self::Cxx23 => "c++23",
        }
    }
}

fn parse_standard(value: &str) -> Result<CppStandard, String> {
    CppStandard::parse(value)
}

fn parse_percentage(value: &str) -> Result<f64, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| format!("invalid percentage '{value}'"))?;
    if parsed.is_finite() && (0.0..=100.0).contains(&parsed) {
        Ok(parsed)
    } else {
        Err(format!(
            "invalid percentage '{value}'. Expected a value between 0 and 100"
        ))
    }
}
