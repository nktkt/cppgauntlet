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
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Compile and run checks for a single C++ source file.
    Check(CheckArgs),

    /// Inspect external tools needed by CppGauntlet workflows.
    Doctor(DoctorArgs),

    /// Write a starter cppgauntlet.yaml configuration file.
    Init(InitArgs),
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    /// C++ source file to check.
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

    /// Per-command timeout in seconds.
    #[arg(long)]
    pub timeout_seconds: Option<u64>,
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
