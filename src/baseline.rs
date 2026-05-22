use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::cli::{BaselineArgs, BaselineCommands, BaselineUpdateArgs};
use crate::error::AppError;
use crate::report::{
    BaselineSummary, Diagnostic, DiagnosticBaselineStatus, DiagnosticSeverity, Report, StageReport,
};

#[derive(Clone, Debug)]
pub struct Baseline {
    path: PathBuf,
    fingerprints: HashSet<String>,
}

impl Baseline {
    pub fn load(path: &Path) -> Result<Self, AppError> {
        let contents = fs::read_to_string(path).map_err(|source| AppError::ReadBaseline {
            path: path.to_path_buf(),
            source,
        })?;
        let report: Report =
            serde_json::from_str(&contents).map_err(|source| AppError::ParseBaseline {
                path: path.to_path_buf(),
                source,
            })?;

        Ok(Self {
            path: path.to_path_buf(),
            fingerprints: report
                .stages
                .iter()
                .flat_map(|stage| stage.diagnostics.iter())
                .map(fingerprint)
                .collect(),
        })
    }

    pub fn compare(&self, stages: &mut [StageReport]) -> BaselineSummary {
        let mut current_fingerprints = HashSet::new();
        let mut new_fingerprints = HashSet::new();
        let mut new_occurrences = 0;

        for diagnostic in stages
            .iter_mut()
            .flat_map(|stage| stage.diagnostics.iter_mut())
        {
            let fingerprint = fingerprint(diagnostic);
            current_fingerprints.insert(fingerprint.clone());

            if self.fingerprints.contains(&fingerprint) {
                diagnostic.baseline_status = Some(DiagnosticBaselineStatus::Existing);
            } else {
                diagnostic.baseline_status = Some(DiagnosticBaselineStatus::New);
                new_fingerprints.insert(fingerprint);
                new_occurrences += 1;
            }
        }

        let resolved = self.fingerprints.difference(&current_fingerprints).count();

        BaselineSummary {
            path: self.path.clone(),
            baseline_unique_diagnostics: self.fingerprints.len(),
            current_unique_diagnostics: current_fingerprints.len(),
            new_unique_diagnostics: new_fingerprints.len(),
            new_diagnostic_occurrences: new_occurrences,
            resolved_unique_diagnostics: resolved,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct BaselineUpdateReport {
    pub schema_version: u32,
    pub source_report: PathBuf,
    pub output: PathBuf,
    pub diagnostics: usize,
    pub unique_diagnostics: usize,
    pub stages: usize,
}

impl BaselineUpdateReport {
    pub fn render_text(&self) -> String {
        [
            "CppGauntlet Baseline".to_string(),
            "Status: UPDATED".to_string(),
            format!("Source report: {}", self.source_report.display()),
            format!("Output: {}", self.output.display()),
            format!("Diagnostics: {}", self.diagnostics),
            format!("Unique diagnostics: {}", self.unique_diagnostics),
            format!("Stages: {}", self.stages),
        ]
        .join("\n")
    }

    pub fn render_markdown(&self) -> String {
        [
            "# CppGauntlet Baseline".to_string(),
            String::new(),
            "| Field | Value |".to_string(),
            "| --- | --- |".to_string(),
            "| Status | updated |".to_string(),
            format!(
                "| Source report | `{}` |",
                markdown_cell(&self.source_report.display().to_string())
            ),
            format!(
                "| Output | `{}` |",
                markdown_cell(&self.output.display().to_string())
            ),
            format!("| Diagnostics | {} |", self.diagnostics),
            format!("| Unique diagnostics | {} |", self.unique_diagnostics),
            format!("| Stages | {} |", self.stages),
        ]
        .join("\n")
    }

    pub fn render_html(&self) -> String {
        format!(
            r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>CppGauntlet Baseline</title>
<style>
body {{ font-family: system-ui, sans-serif; margin: 2rem; color: #17202a; background: #f7f8fa; }}
main {{ max-width: 920px; margin: 0 auto; background: #fff; border: 1px solid #d7dde5; padding: 1.5rem; }}
table {{ border-collapse: collapse; width: 100%; }}
th, td {{ border-bottom: 1px solid #d7dde5; padding: 0.55rem; text-align: left; }}
code {{ background: #eef1f5; padding: 0.1rem 0.25rem; }}
</style>
</head>
<body>
<main>
<h1>CppGauntlet Baseline</h1>
<table>
<tbody>
<tr><th>Status</th><td>updated</td></tr>
<tr><th>Source report</th><td><code>{}</code></td></tr>
<tr><th>Output</th><td><code>{}</code></td></tr>
<tr><th>Diagnostics</th><td>{}</td></tr>
<tr><th>Unique diagnostics</th><td>{}</td></tr>
<tr><th>Stages</th><td>{}</td></tr>
</tbody>
</table>
</main>
</body>
</html>"#,
            html_escape(&self.source_report.display().to_string()),
            html_escape(&self.output.display().to_string()),
            self.diagnostics,
            self.unique_diagnostics,
            self.stages
        )
    }
}

pub fn run(args: BaselineArgs) -> Result<BaselineUpdateReport, AppError> {
    match args.command {
        BaselineCommands::Update(args) => update(args),
    }
}

fn update(args: BaselineUpdateArgs) -> Result<BaselineUpdateReport, AppError> {
    let mut report = read_report(&args.report)?;
    let diagnostics = report
        .stages
        .iter()
        .map(|stage| stage.diagnostics.len())
        .sum();
    let unique_diagnostics = unique_diagnostic_count(&report);
    let stages = report.stages.len();

    normalize_report_for_baseline(&mut report, &args.output);
    write_baseline(&args.output, &report)?;

    Ok(BaselineUpdateReport {
        schema_version: 1,
        source_report: args.report,
        output: args.output,
        diagnostics,
        unique_diagnostics,
        stages,
    })
}

fn read_report(path: &Path) -> Result<Report, AppError> {
    let contents = fs::read_to_string(path).map_err(|source| AppError::ReadReport {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&contents).map_err(|source| AppError::ParseReport {
        path: path.to_path_buf(),
        source,
    })
}

fn normalize_report_for_baseline(report: &mut Report, output: &Path) {
    report.report_path = output.to_path_buf();
    report.markdown_report_path = None;
    report.html_report_path = None;
    report.sarif_report_path = None;
    report.summary.baseline = None;

    for diagnostic in report
        .stages
        .iter_mut()
        .flat_map(|stage| stage.diagnostics.iter_mut())
    {
        diagnostic.baseline_status = None;
    }
}

fn write_baseline(path: &Path, report: &Report) -> Result<(), AppError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|source| AppError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let serialized = serde_json::to_string_pretty(report)?;
    fs::write(path, serialized).map_err(|source| AppError::WriteBaseline {
        path: path.to_path_buf(),
        source,
    })
}

fn unique_diagnostic_count(report: &Report) -> usize {
    report
        .stages
        .iter()
        .flat_map(|stage| stage.diagnostics.iter())
        .map(fingerprint)
        .collect::<HashSet<_>>()
        .len()
}

fn fingerprint(diagnostic: &Diagnostic) -> String {
    format!(
        "{}\0{}\0{}",
        severity_key(diagnostic.severity),
        normalize(&diagnostic.message),
        normalize(&diagnostic.raw)
    )
}

fn severity_key(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Error => "error",
    }
}

fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
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
