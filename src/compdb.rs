use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::AppError;

#[derive(Clone, Debug)]
pub struct CompilationDatabase {
    pub path: PathBuf,
    pub entries: Vec<CompilationUnit>,
}

#[derive(Clone, Debug)]
pub struct CompilationUnit {
    pub directory: PathBuf,
    pub file: PathBuf,
    pub arguments: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawCompilationUnit {
    directory: PathBuf,
    file: PathBuf,
    #[serde(default)]
    arguments: Vec<String>,
    #[serde(default)]
    command: Option<String>,
}

pub fn load_for_target(target: &Path) -> Result<CompilationDatabase, AppError> {
    let path = resolve_database_path(target)?;
    let contents =
        fs::read_to_string(&path).map_err(|source| AppError::ReadCompilationDatabase {
            path: path.clone(),
            source,
        })?;
    let raw_entries: Vec<RawCompilationUnit> =
        serde_json::from_str(&contents).map_err(|source| AppError::ParseCompilationDatabase {
            path: path.clone(),
            source,
        })?;

    if raw_entries.is_empty() {
        return Err(AppError::EmptyCompilationDatabase(path));
    }

    let entries = raw_entries
        .into_iter()
        .map(CompilationUnit::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(CompilationDatabase { path, entries })
}

fn resolve_database_path(target: &Path) -> Result<PathBuf, AppError> {
    if target.is_file()
        && target
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "compile_commands.json")
    {
        return Ok(target.to_path_buf());
    }

    if !target.is_dir() {
        return Err(AppError::UnsupportedCheckTarget(target.to_path_buf()));
    }

    for candidate in [
        target.join("compile_commands.json"),
        target.join("build").join("compile_commands.json"),
    ] {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(AppError::CompilationDatabaseMissing(target.to_path_buf()))
}

impl TryFrom<RawCompilationUnit> for CompilationUnit {
    type Error = AppError;

    fn try_from(raw: RawCompilationUnit) -> Result<Self, Self::Error> {
        let arguments = if raw.arguments.is_empty() {
            match raw.command {
                Some(command) => split_command(&command),
                None => Vec::new(),
            }
        } else {
            raw.arguments
        };

        if arguments.is_empty() {
            return Err(AppError::InvalidCompilationCommand(raw.file));
        }

        Ok(Self {
            directory: raw.directory,
            file: raw.file,
            arguments,
        })
    }
}

fn split_command(command: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = command.chars().peekable();
    let mut quote = None;

    while let Some(ch) = chars.next() {
        match ch {
            '\'' | '"' if quote.is_none() => quote = Some(ch),
            '\'' | '"' if quote == Some(ch) => quote = None,
            '\\' => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            ch if ch.is_whitespace() && quote.is_none() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}
