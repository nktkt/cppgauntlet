use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::cli::{CppStandard, InitArgs};
use crate::error::AppError;

pub const DEFAULT_CONFIG_PATH: &str = "cppgauntlet.yaml";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compiler: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_dir: Option<PathBuf>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sanitizers: Option<SanitizerConfig>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<ReportConfig>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<TestConfig>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub static_analysis: Option<StaticAnalysisConfig>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<CoverageConfig>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<BaselineConfig>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<PolicyConfig>,
}

impl ProjectConfig {
    pub fn starter() -> Self {
        Self {
            standard: Some("c++20".to_string()),
            compiler: Some("clang++".to_string()),
            artifact_dir: Some(PathBuf::from(".cppgauntlet")),
            timeout_seconds: Some(30),
            sanitizers: Some(SanitizerConfig {
                enabled: vec!["address".to_string(), "undefined".to_string()],
            }),
            report: Some(ReportConfig {
                path: Some(PathBuf::from(".cppgauntlet/cppgauntlet-report.json")),
                markdown_path: None,
                html_path: None,
            }),
            test: Some(TestConfig {
                ctest: Some(false),
                command: None,
            }),
            static_analysis: Some(StaticAnalysisConfig {
                clang_tidy: Some(false),
                clang_tidy_bin: Some("clang-tidy".to_string()),
                clang_tidy_checks: None,
            }),
            coverage: Some(CoverageConfig {
                enabled: Some(false),
                llvm_cov_bin: Some("llvm-cov".to_string()),
                llvm_profdata_bin: Some("llvm-profdata".to_string()),
            }),
            baseline: Some(BaselineConfig { path: None }),
            policy: Some(PolicyConfig {
                max_warnings: None,
                min_line_coverage: None,
                fail_on_new_diagnostics: Some(false),
            }),
        }
    }

    pub fn standard(&self) -> Result<Option<CppStandard>, AppError> {
        self.standard
            .as_deref()
            .map(CppStandard::parse)
            .transpose()
            .map_err(AppError::InvalidStandard)
    }

    pub fn sanitizers_csv(&self) -> Option<String> {
        self.sanitizers.as_ref().map(|sanitizers| {
            if sanitizers.enabled.is_empty() {
                "none".to_string()
            } else {
                sanitizers.enabled.join(",")
            }
        })
    }

    pub fn report_path(&self) -> Option<PathBuf> {
        self.report.as_ref().and_then(|report| report.path.clone())
    }

    pub fn markdown_report_path(&self) -> Option<PathBuf> {
        self.report
            .as_ref()
            .and_then(|report| report.markdown_path.clone())
    }

    pub fn html_report_path(&self) -> Option<PathBuf> {
        self.report
            .as_ref()
            .and_then(|report| report.html_path.clone())
    }

    pub fn ctest_enabled(&self) -> Option<bool> {
        self.test.as_ref().and_then(|test| test.ctest)
    }

    pub fn test_command(&self) -> Option<String> {
        self.test.as_ref().and_then(|test| test.command.clone())
    }

    pub fn clang_tidy_enabled(&self) -> Option<bool> {
        self.static_analysis
            .as_ref()
            .and_then(|analysis| analysis.clang_tidy)
    }

    pub fn clang_tidy_bin(&self) -> Option<String> {
        self.static_analysis
            .as_ref()
            .and_then(|analysis| analysis.clang_tidy_bin.clone())
    }

    pub fn clang_tidy_checks(&self) -> Option<String> {
        self.static_analysis
            .as_ref()
            .and_then(|analysis| analysis.clang_tidy_checks.clone())
    }

    pub fn coverage_enabled(&self) -> Option<bool> {
        self.coverage.as_ref().and_then(|coverage| coverage.enabled)
    }

    pub fn llvm_cov_bin(&self) -> Option<String> {
        self.coverage
            .as_ref()
            .and_then(|coverage| coverage.llvm_cov_bin.clone())
    }

    pub fn llvm_profdata_bin(&self) -> Option<String> {
        self.coverage
            .as_ref()
            .and_then(|coverage| coverage.llvm_profdata_bin.clone())
    }

    pub fn baseline_path(&self) -> Option<PathBuf> {
        self.baseline
            .as_ref()
            .and_then(|baseline| baseline.path.clone())
    }

    pub fn max_warnings(&self) -> Option<usize> {
        self.policy.as_ref().and_then(|policy| policy.max_warnings)
    }

    pub fn min_line_coverage(&self) -> Option<f64> {
        self.policy
            .as_ref()
            .and_then(|policy| policy.min_line_coverage)
    }

    pub fn fail_on_new_diagnostics(&self) -> Option<bool> {
        self.policy
            .as_ref()
            .and_then(|policy| policy.fail_on_new_diagnostics)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SanitizerConfig {
    #[serde(default)]
    pub enabled: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReportConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub markdown_path: Option<PathBuf>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TestConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ctest: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StaticAnalysisConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clang_tidy: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clang_tidy_bin: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clang_tidy_checks: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llvm_cov_bin: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llvm_profdata_bin: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_warnings: Option<usize>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_line_coverage: Option<f64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fail_on_new_diagnostics: Option<bool>,
}

pub fn load_config(requested_path: Option<&Path>) -> Result<ProjectConfig, AppError> {
    match requested_path {
        Some(path) => load_existing_config(path),
        None => {
            let default_path = Path::new(DEFAULT_CONFIG_PATH);
            if default_path.exists() {
                load_existing_config(default_path)
            } else {
                Ok(ProjectConfig::default())
            }
        }
    }
}

pub fn init(args: InitArgs) -> Result<PathBuf, AppError> {
    if args.path.exists() && !args.force {
        return Err(AppError::ConfigExists(args.path));
    }

    if let Some(parent) = args
        .path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|source| AppError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let serialized = serde_yml::to_string(&ProjectConfig::starter())?;
    fs::write(&args.path, serialized).map_err(|source| AppError::WriteConfig {
        path: args.path.clone(),
        source,
    })?;

    Ok(args.path)
}

fn load_existing_config(path: &Path) -> Result<ProjectConfig, AppError> {
    let contents = fs::read_to_string(path).map_err(|source| AppError::ReadConfig {
        path: path.to_path_buf(),
        source,
    })?;

    serde_yml::from_str(&contents).map_err(|source| AppError::ParseConfig {
        path: path.to_path_buf(),
        source,
    })
}
