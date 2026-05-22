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
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    /// C++ source file to check.
    pub file: PathBuf,

    /// C++ language standard to pass to the compiler.
    #[arg(long, default_value = "c++20", value_parser = parse_standard)]
    pub standard: CppStandard,

    /// C++ compiler executable.
    #[arg(long, default_value = "clang++")]
    pub compiler: String,

    /// Comma-separated sanitizer list: address, undefined, asan, ubsan, or none.
    #[arg(long, default_value = "address,undefined")]
    pub sanitizers: String,

    /// Root directory for generated artifacts and reports.
    #[arg(long, default_value = ".cppgauntlet")]
    pub artifact_dir: PathBuf,

    /// Optional report path. Defaults to <artifact-dir>/cppgauntlet-report.json.
    #[arg(long)]
    pub report: Option<PathBuf>,

    /// Per-command timeout in seconds.
    #[arg(long, default_value_t = 30)]
    pub timeout_seconds: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CppStandard {
    Cxx17,
    Cxx20,
    Cxx23,
}

impl CppStandard {
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
    match value {
        "c++17" | "17" => Ok(CppStandard::Cxx17),
        "c++20" | "20" => Ok(CppStandard::Cxx20),
        "c++23" | "23" => Ok(CppStandard::Cxx23),
        _ => Err(format!(
            "unsupported C++ standard '{value}'. Expected c++17, c++20, or c++23"
        )),
    }
}
