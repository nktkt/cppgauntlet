use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use wait_timeout::ChildExt;

use crate::error::AppError;

#[derive(Clone, Debug)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub current_dir: Option<PathBuf>,
}

impl CommandSpec {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: None,
        }
    }

    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn command_line(&self) -> Vec<String> {
        let mut parts = Vec::with_capacity(self.args.len() + 1);
        parts.push(self.program.clone());
        parts.extend(self.args.clone());
        parts
    }
}

#[derive(Clone, Debug)]
pub struct CommandResult {
    pub command: Vec<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

impl CommandResult {
    pub fn success(&self) -> bool {
        self.exit_code == Some(0) && !self.timed_out
    }
}

pub fn run_command(spec: CommandSpec, timeout: Duration) -> Result<CommandResult, AppError> {
    let command_line = spec.command_line();
    let mut command = Command::new(&spec.program);
    command.args(&spec.args);
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    if let Some(current_dir) = &spec.current_dir {
        command.current_dir(current_dir);
    }

    let mut child = command.spawn().map_err(|source| AppError::CommandSpawn {
        program: spec.program.clone(),
        source,
    })?;

    let timed_out = match child
        .wait_timeout(timeout)
        .map_err(|source| AppError::CommandWait {
            program: spec.program.clone(),
            source,
        })? {
        Some(_) => false,
        None => {
            let _ = child.kill();
            true
        }
    };

    let output = child
        .wait_with_output()
        .map_err(|source| AppError::CommandWait {
            program: spec.program.clone(),
            source,
        })?;

    Ok(CommandResult {
        command: command_line,
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        timed_out,
    })
}
