use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

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
