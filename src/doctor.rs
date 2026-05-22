use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::cli::DoctorArgs;
use crate::runner::{run_command, CommandSpec};

const DEFAULT_REQUIRED_TOOLS: &[&str] = &["clang++"];
const DEFAULT_OPTIONAL_TOOLS: &[&str] = &[
    "clang-tidy",
    "llvm-cov",
    "llvm-profdata",
    "cmake",
    "ctest",
    "ninja",
];

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DoctorReport {
    pub schema_version: u32,
    pub status: DoctorStatus,
    pub required_missing: Vec<String>,
    pub tools: Vec<ToolCheck>,
}

impl DoctorReport {
    pub fn is_success(&self) -> bool {
        self.status == DoctorStatus::Passed
    }

    pub fn render_text(&self) -> String {
        let mut lines = vec![
            "CppGauntlet Doctor".to_string(),
            format!("Status: {}", self.status.as_str().to_uppercase()),
            format!(
                "Required missing: {}",
                if self.required_missing.is_empty() {
                    "none".to_string()
                } else {
                    self.required_missing.join(", ")
                }
            ),
            String::new(),
            "Tools:".to_string(),
        ];

        for tool in &self.tools {
            let required = if tool.required {
                "required"
            } else {
                "optional"
            };
            let availability = if tool.available { "found" } else { "missing" };
            let detail = tool
                .version
                .as_deref()
                .or(tool.error.as_deref())
                .unwrap_or("no details");

            lines.push(format!(
                "- {}: {} ({}, {})",
                tool.name, availability, required, detail
            ));
        }

        lines.join("\n")
    }

    pub fn render_markdown(&self) -> String {
        let mut lines = vec![
            "# CppGauntlet Doctor".to_string(),
            String::new(),
            "## Summary".to_string(),
            String::new(),
            "| Field | Value |".to_string(),
            "| --- | --- |".to_string(),
            format!("| Status | {} |", self.status.as_str()),
            format!(
                "| Required missing | {} |",
                if self.required_missing.is_empty() {
                    "none".to_string()
                } else {
                    self.required_missing.join(", ")
                }
            ),
            String::new(),
            "## Tools".to_string(),
            String::new(),
            "| Tool | Required | Available | Detail |".to_string(),
            "| --- | --- | --- | --- |".to_string(),
        ];

        for tool in &self.tools {
            let detail = tool
                .version
                .as_deref()
                .or(tool.error.as_deref())
                .unwrap_or("no details");

            lines.push(format!(
                "| {} | {} | {} | {} |",
                markdown_cell(&tool.name),
                tool.required,
                tool.available,
                markdown_cell(detail)
            ));
        }

        lines.join("\n")
    }

    pub fn render_html(&self) -> String {
        let mut rows = String::new();
        for tool in &self.tools {
            let detail = tool
                .version
                .as_deref()
                .or(tool.error.as_deref())
                .unwrap_or("no details");
            rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                html_escape(&tool.name),
                tool.required,
                tool.available,
                html_escape(detail)
            ));
        }

        format!(
            r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>CppGauntlet Doctor</title>
<style>
body {{ font-family: system-ui, sans-serif; margin: 2rem; color: #17202a; background: #f7f8fa; }}
main {{ max-width: 980px; margin: 0 auto; background: #fff; border: 1px solid #d7dde5; padding: 1.5rem; }}
table {{ border-collapse: collapse; width: 100%; }}
th, td {{ border-bottom: 1px solid #d7dde5; padding: 0.55rem; text-align: left; }}
</style>
</head>
<body>
<main>
<h1>CppGauntlet Doctor</h1>
<p>Status: <strong>{}</strong></p>
<p>Required missing: {}</p>
<h2>Tools</h2>
<table>
<thead><tr><th>Tool</th><th>Required</th><th>Available</th><th>Detail</th></tr></thead>
<tbody>{}</tbody>
</table>
</main>
</body>
</html>"#,
            self.status.as_str(),
            if self.required_missing.is_empty() {
                "none".to_string()
            } else {
                html_escape(&self.required_missing.join(", "))
            },
            rows
        )
    }
}

fn markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorStatus {
    Passed,
    Failed,
}

impl DoctorStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolCheck {
    pub name: String,
    pub required: bool,
    pub available: bool,
    pub command: Vec<String>,
    pub exit_code: Option<i32>,
    pub version: Option<String>,
    pub error: Option<String>,
}

pub fn run(args: DoctorArgs) -> Result<DoctorReport, crate::error::AppError> {
    let tools = resolve_tools(&args);
    let timeout = Duration::from_secs(args.timeout_seconds);

    let tool_checks = tools
        .into_iter()
        .map(|tool| check_tool(tool, timeout))
        .collect::<Vec<_>>();

    let required_missing = tool_checks
        .iter()
        .filter(|tool| tool.required && !tool.available)
        .map(|tool| tool.name.clone())
        .collect::<Vec<_>>();

    let status = if required_missing.is_empty() {
        DoctorStatus::Passed
    } else {
        DoctorStatus::Failed
    };

    Ok(DoctorReport {
        schema_version: 1,
        status,
        required_missing,
        tools: tool_checks,
    })
}

#[derive(Clone, Debug)]
struct ToolRequest {
    name: String,
    required: bool,
}

fn resolve_tools(args: &DoctorArgs) -> Vec<ToolRequest> {
    if args.required_tools.is_empty() && args.optional_tools.is_empty() {
        return DEFAULT_REQUIRED_TOOLS
            .iter()
            .map(|name| ToolRequest {
                name: (*name).to_string(),
                required: true,
            })
            .chain(DEFAULT_OPTIONAL_TOOLS.iter().map(|name| ToolRequest {
                name: (*name).to_string(),
                required: false,
            }))
            .collect();
    }

    args.required_tools
        .iter()
        .map(|name| ToolRequest {
            name: name.clone(),
            required: true,
        })
        .chain(args.optional_tools.iter().map(|name| ToolRequest {
            name: name.clone(),
            required: false,
        }))
        .collect()
}

fn check_tool(tool: ToolRequest, timeout: Duration) -> ToolCheck {
    let spec = CommandSpec::new(&tool.name).args(["--version"]);
    let command = spec.command_line();

    match run_command(spec, timeout) {
        Ok(result) => {
            let version = first_output_line(&result.stdout, &result.stderr);
            let available = result.success();
            let error = if available {
                None
            } else if result.timed_out {
                Some("version check timed out".to_string())
            } else {
                Some(format!(
                    "version check exited with {}",
                    result
                        .exit_code
                        .map(|code| code.to_string())
                        .unwrap_or_else(|| "unknown status".to_string())
                ))
            };

            ToolCheck {
                name: tool.name,
                required: tool.required,
                available,
                command: result.command,
                exit_code: result.exit_code,
                version,
                error,
            }
        }
        Err(error) => ToolCheck {
            name: tool.name,
            required: tool.required,
            available: false,
            command,
            exit_code: None,
            version: None,
            error: Some(error.to_string()),
        },
    }
}

fn first_output_line(stdout: &str, stderr: &str) -> Option<String> {
    stdout
        .lines()
        .chain(stderr.lines())
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}
