use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("target path does not exist: {0}")]
    TargetMissing(PathBuf),

    #[error("unsupported check target: {0}. Expected a C++ source file, a directory containing compile_commands.json, or a compile_commands.json file")]
    UnsupportedCheckTarget(PathBuf),

    #[error("could not find compile_commands.json under: {0}")]
    CompilationDatabaseMissing(PathBuf),

    #[error("failed to read compilation database {path}: {source}")]
    ReadCompilationDatabase {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse compilation database {path}: {source}")]
    ParseCompilationDatabase {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("compilation database is empty: {0}")]
    EmptyCompilationDatabase(PathBuf),

    #[error("compilation database entry has no executable command for file: {0}")]
    InvalidCompilationCommand(PathBuf),

    #[error("--ctest is only supported when checking a CMake project directory")]
    CtestRequiresCMakeProject,

    #[error("invalid sanitizer '{0}'. Expected address, undefined, asan, ubsan, or none")]
    InvalidSanitizer(String),

    #[error("{0}")]
    InvalidStandard(String),

    #[error("configuration file already exists: {0}. Use --force to overwrite it")]
    ConfigExists(PathBuf),

    #[error("failed to create directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to read configuration {path}: {source}")]
    ReadConfig {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse configuration {path}: {source}")]
    ParseConfig {
        path: PathBuf,
        source: serde_yml::Error,
    },

    #[error("failed to write configuration {path}: {source}")]
    WriteConfig {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to write report {path}: {source}")]
    WriteReport {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to run command '{program}': {source}")]
    CommandSpawn {
        program: String,
        source: std::io::Error,
    },

    #[error("failed while waiting for command '{program}': {source}")]
    CommandWait {
        program: String,
        source: std::io::Error,
    },

    #[error("failed to serialize report: {0}")]
    Json(#[from] serde_json::Error),

    #[error("failed to serialize configuration: {0}")]
    Yaml(#[from] serde_yml::Error),
}
