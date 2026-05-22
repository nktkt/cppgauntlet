mod check;
mod cli;
mod error;
mod report;
mod runner;

use clap::Parser;
use cli::{Cli, Commands, OutputFormat};
use error::AppError;

fn main() {
    let cli = Cli::parse();

    let exit_code = match run(cli) {
        Ok(success) => {
            if success {
                0
            } else {
                1
            }
        }
        Err(error) => {
            eprintln!("error: {error}");
            2
        }
    };

    std::process::exit(exit_code);
}

fn run(cli: Cli) -> Result<bool, AppError> {
    match cli.command {
        Commands::Check(args) => {
            let report = check::run(args)?;
            let success = report.is_success();

            match cli.format {
                OutputFormat::Text => {
                    println!("{}", report.render_text());
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
            }

            Ok(success)
        }
    }
}
