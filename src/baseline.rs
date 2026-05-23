use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::cli::{BaselineArgs, BaselineCommands, BaselineUpdateArgs};
use crate::error::AppError;
use crate::report::{
    parse_diagnostics, BaselineSummary, Diagnostic, DiagnosticBaselineStatus, Report, StageReport,
    REPORT_SCHEMA_VERSION,
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
            fingerprints: diagnostic_fingerprints(&report),
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_baseline: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_unique_diagnostics: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_unique_diagnostics: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_unique_diagnostics: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unchanged_unique_diagnostics: Option<usize>,
}

impl BaselineUpdateReport {
    pub fn render_text(&self) -> String {
        let mut lines = vec![
            "CppGauntlet Baseline".to_string(),
            "Status: UPDATED".to_string(),
            format!("Source report: {}", self.source_report.display()),
            format!("Output: {}", self.output.display()),
            format!("Diagnostics: {}", self.diagnostics),
            format!("Unique diagnostics: {}", self.unique_diagnostics),
            format!("Stages: {}", self.stages),
        ];
        self.push_text_change_summary(&mut lines);
        lines.join("\n")
    }

    pub fn render_markdown(&self) -> String {
        let mut rows = vec![
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
        ];
        self.push_markdown_change_summary(&mut rows);
        rows.join("\n")
    }

    pub fn render_html(&self) -> String {
        let change_summary = self.render_html_change_summary();

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
{}
</tbody>
</table>
</main>
</body>
</html>"#,
            html_escape(&self.source_report.display().to_string()),
            html_escape(&self.output.display().to_string()),
            self.diagnostics,
            self.unique_diagnostics,
            self.stages,
            change_summary
        )
    }

    fn push_text_change_summary(&self, lines: &mut Vec<String>) {
        if let Some(previous_baseline) = &self.previous_baseline {
            lines.push(format!(
                "Previous baseline: {}",
                previous_baseline.display()
            ));
        }
        if let Some(value) = self.previous_unique_diagnostics {
            lines.push(format!("Previous unique diagnostics: {value}"));
        }
        if let Some(value) = self.new_unique_diagnostics {
            lines.push(format!("New unique diagnostics: {value}"));
        }
        if let Some(value) = self.resolved_unique_diagnostics {
            lines.push(format!("Resolved unique diagnostics: {value}"));
        }
        if let Some(value) = self.unchanged_unique_diagnostics {
            lines.push(format!("Unchanged unique diagnostics: {value}"));
        }
    }

    fn push_markdown_change_summary(&self, rows: &mut Vec<String>) {
        if let Some(previous_baseline) = &self.previous_baseline {
            rows.push(format!(
                "| Previous baseline | `{}` |",
                markdown_cell(&previous_baseline.display().to_string())
            ));
        }
        if let Some(value) = self.previous_unique_diagnostics {
            rows.push(format!("| Previous unique diagnostics | {value} |"));
        }
        if let Some(value) = self.new_unique_diagnostics {
            rows.push(format!("| New unique diagnostics | {value} |"));
        }
        if let Some(value) = self.resolved_unique_diagnostics {
            rows.push(format!("| Resolved unique diagnostics | {value} |"));
        }
        if let Some(value) = self.unchanged_unique_diagnostics {
            rows.push(format!("| Unchanged unique diagnostics | {value} |"));
        }
    }

    fn render_html_change_summary(&self) -> String {
        let mut rows = Vec::new();

        if let Some(previous_baseline) = &self.previous_baseline {
            rows.push(format!(
                "<tr><th>Previous baseline</th><td><code>{}</code></td></tr>",
                html_escape(&previous_baseline.display().to_string())
            ));
        }
        if let Some(value) = self.previous_unique_diagnostics {
            rows.push(format!(
                "<tr><th>Previous unique diagnostics</th><td>{value}</td></tr>"
            ));
        }
        if let Some(value) = self.new_unique_diagnostics {
            rows.push(format!(
                "<tr><th>New unique diagnostics</th><td>{value}</td></tr>"
            ));
        }
        if let Some(value) = self.resolved_unique_diagnostics {
            rows.push(format!(
                "<tr><th>Resolved unique diagnostics</th><td>{value}</td></tr>"
            ));
        }
        if let Some(value) = self.unchanged_unique_diagnostics {
            rows.push(format!(
                "<tr><th>Unchanged unique diagnostics</th><td>{value}</td></tr>"
            ));
        }

        rows.join("\n")
    }
}

pub fn run(args: BaselineArgs) -> Result<BaselineUpdateReport, AppError> {
    match args.command {
        BaselineCommands::Update(args) => update(args),
    }
}

fn update(args: BaselineUpdateArgs) -> Result<BaselineUpdateReport, AppError> {
    let mut report = read_report(&args.report)?;
    normalize_report_for_baseline(&mut report, &args.output);

    let diagnostics = report
        .stages
        .iter()
        .map(|stage| stage.diagnostics.len())
        .sum();
    let current_fingerprints = diagnostic_fingerprints(&report);
    let unique_diagnostics = current_fingerprints.len();
    let stages = report.stages.len();
    let change_summary = args
        .previous
        .as_deref()
        .map(|path| baseline_change_summary(path, &current_fingerprints))
        .transpose()?;

    write_baseline(&args.output, &report)?;

    let (
        previous_baseline,
        previous_unique_diagnostics,
        new_unique_diagnostics,
        resolved_unique_diagnostics,
        unchanged_unique_diagnostics,
    ) = change_summary.map_or((None, None, None, None, None), |summary| {
        (
            Some(summary.previous_baseline),
            Some(summary.previous_unique_diagnostics),
            Some(summary.new_unique_diagnostics),
            Some(summary.resolved_unique_diagnostics),
            Some(summary.unchanged_unique_diagnostics),
        )
    });

    Ok(BaselineUpdateReport {
        schema_version: 1,
        source_report: args.report,
        output: args.output,
        diagnostics,
        unique_diagnostics,
        stages,
        previous_baseline,
        previous_unique_diagnostics,
        new_unique_diagnostics,
        resolved_unique_diagnostics,
        unchanged_unique_diagnostics,
    })
}

#[derive(Debug)]
struct BaselineChangeSummary {
    previous_baseline: PathBuf,
    previous_unique_diagnostics: usize,
    new_unique_diagnostics: usize,
    resolved_unique_diagnostics: usize,
    unchanged_unique_diagnostics: usize,
}

fn baseline_change_summary(
    previous_path: &Path,
    current_fingerprints: &HashSet<String>,
) -> Result<BaselineChangeSummary, AppError> {
    let previous = Baseline::load(previous_path)?;
    let new_unique_diagnostics = current_fingerprints
        .difference(&previous.fingerprints)
        .count();
    let resolved_unique_diagnostics = previous
        .fingerprints
        .difference(current_fingerprints)
        .count();
    let unchanged_unique_diagnostics = current_fingerprints
        .intersection(&previous.fingerprints)
        .count();

    Ok(BaselineChangeSummary {
        previous_baseline: previous.path,
        previous_unique_diagnostics: previous.fingerprints.len(),
        new_unique_diagnostics,
        resolved_unique_diagnostics,
        unchanged_unique_diagnostics,
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
    report.schema_version = REPORT_SCHEMA_VERSION;
    report.report_path = output.to_path_buf();
    report.markdown_report_path = None;
    report.html_report_path = None;
    report.sarif_report_path = None;
    report.summary.baseline = None;

    for stage in &mut report.stages {
        if stage.diagnostics.is_empty() {
            stage.diagnostics = parse_diagnostics(&format!("{}\n{}", stage.stdout, stage.stderr));
        }
    }

    for diagnostic in report
        .stages
        .iter_mut()
        .flat_map(|stage| stage.diagnostics.iter_mut())
    {
        diagnostic.ensure_metadata();
        diagnostic.baseline_status = None;
    }

    report.summary.diagnostics = report
        .stages
        .iter()
        .map(|stage| stage.diagnostics.len())
        .sum();
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

fn diagnostic_fingerprints(report: &Report) -> HashSet<String> {
    report
        .stages
        .iter()
        .flat_map(|stage| stage.diagnostics.iter())
        .map(fingerprint)
        .collect::<HashSet<_>>()
}

fn fingerprint(diagnostic: &Diagnostic) -> String {
    diagnostic.stable_fingerprint()
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
