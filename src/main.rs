mod baseline;
mod check;
mod cli;
mod compdb;
mod config;
mod doctor;
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
                OutputFormat::Markdown => {
                    println!("{}", report.render_markdown());
                }
                OutputFormat::Html => {
                    println!("{}", report.render_html());
                }
            }

            Ok(success)
        }
        Commands::Baseline(args) => {
            let report = baseline::run(args)?;

            match cli.format {
                OutputFormat::Text => {
                    println!("{}", report.render_text());
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                OutputFormat::Markdown => {
                    println!("{}", report.render_markdown());
                }
                OutputFormat::Html => {
                    println!("{}", report.render_html());
                }
            }

            Ok(true)
        }
        Commands::Doctor(args) => {
            let report = doctor::run(args)?;
            let success = report.is_success();

            match cli.format {
                OutputFormat::Text => {
                    println!("{}", report.render_text());
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                OutputFormat::Markdown => {
                    println!("{}", report.render_markdown());
                }
                OutputFormat::Html => {
                    println!("{}", report.render_html());
                }
            }

            Ok(success)
        }
        Commands::Init(args) => {
            let path = config::init(args)?;

            match cli.format {
                OutputFormat::Text => {
                    println!("Created {}", path.display());
                }
                OutputFormat::Json => {
                    println!(
                        "{}",
                        serde_json::json!({
                            "created": path,
                        })
                    );
                }
                OutputFormat::Markdown => {
                    println!("# CppGauntlet Init\n\nCreated `{}`.", path.display());
                }
                OutputFormat::Html => {
                    println!(
                        "<!doctype html><html><head><meta charset=\"utf-8\"><title>CppGauntlet Init</title></head><body><h1>CppGauntlet Init</h1><p>Created <code>{}</code>.</p></body></html>",
                        path.display()
                    );
                }
            }

            Ok(true)
        }
    }
}
