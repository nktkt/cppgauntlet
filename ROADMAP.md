# CppGauntlet Roadmap

CppGauntlet is being built as a long-term, scalable C++ verification product, not a one-off wrapper around compiler commands.

The roadmap is organized around product maturity: first make the local developer loop useful, then make the results reliable in CI, then scale to teams, repositories, and extensible analysis workflows.

## Product Principles

- **Useful by default:** A new user should get meaningful results from one command.
- **Transparent execution:** Every compiler flag, external tool, and generated artifact should be inspectable.
- **Composable pipeline:** Compile, analyze, test, sanitize, fuzz, and report steps should be independent modules.
- **Deterministic reports:** Re-running the same configuration should produce comparable output.
- **CI-first design:** Exit codes, JSON output, and artifact paths should work well in automation.
- **Extensible core:** New analyzers, build systems, and report formats should be pluggable without rewriting the CLI.
- **Local-first, cloud-ready:** The product should work fully offline first, while leaving room for hosted dashboards later.

## Phase 0: Foundation

Goal: establish the project shape, Rust implementation baseline, and contribution surface.

Planned work:

- Create the Rust workspace and CLI skeleton.
- Define command names, arguments, and exit code behavior.
- Add structured error handling.
- Add logging with quiet, normal, and verbose modes.
- Add integration test fixtures for small C++ programs.
- Add GitHub Actions for Rust formatting, linting, and tests.
- Add license, contributing guide, code of conduct, and issue templates.

Exit criteria:

- `cppgauntlet --help` works.
- The project builds on macOS and Linux.
- Contributors can run the full Rust test suite locally.

## Phase 1: Single-File Gauntlet

Goal: make CppGauntlet useful for standalone C++ files.

Planned work:

- Implement `cppgauntlet check <file>`.
- Support C++17, C++20, and C++23.
- Compile with `clang++`.
- Add strict warning presets.
- Capture diagnostics into structured JSON.
- Build and run sanitizer variants.
- Support AddressSanitizer and UndefinedBehaviorSanitizer.
- Emit terminal summaries and JSON reports.
- Store artifacts under a predictable `.cppgauntlet/` directory.

Exit criteria:

- A user can run one command on `main.cpp`.
- Compile failures, warnings, sanitizer failures, and runtime exit codes are all reported clearly.
- JSON output is stable enough for early CI usage.

## Phase 2: Project Detection

Goal: move from single files to real repositories.

Planned work:

- Detect CMake projects.
- Detect `compile_commands.json`.
- Add `cppgauntlet init` to generate `cppgauntlet.yaml`.
- Add configuration loading and validation.
- Add include path and compiler flag discovery.
- Support custom test commands.
- Add timeout controls.
- Add artifact cleanup controls.

Exit criteria:

- CppGauntlet can inspect a small CMake project without manual compiler flag copying.
- Configuration errors are actionable.
- Existing project build settings remain the source of truth.

## Phase 3: Static Analysis and Test Integration

Goal: combine compiler diagnostics, static analysis, and test execution into one report.

Planned work:

- Add `clang-tidy` integration.
- Add Clang Static Analyzer integration.
- Add `ctest` integration.
- Parse common test result outputs.
- Normalize issue severity.
- Deduplicate repeated diagnostics.
- Add baseline support to separate new issues from existing debt.
- Add CI-friendly summary output.

Exit criteria:

- Teams can fail CI only on new issues.
- Reports distinguish compiler errors, warnings, analyzer findings, test failures, and sanitizer failures.
- The product remains usable when optional tools are missing.

## Phase 4: Coverage, Fuzzing, and Quality Scoring

Goal: expand from pass/fail checks into measurable quality trends.

Planned work:

- Add `llvm-cov` and `llvm-profdata` support.
- Add line and function coverage summaries.
- Add libFuzzer workflow support.
- Add corpus and crash artifact management.
- Add configurable quality scoring.
- Add trend-friendly JSON schema.
- Add Markdown and HTML reports.

Exit criteria:

- CppGauntlet can produce a quality score that is useful for tracking change over time.
- Coverage and fuzzing results can be published as CI artifacts.
- Reports are readable by humans and stable for machines.

## Phase 5: Scalable Architecture

Goal: make the tool fast, extensible, and maintainable as checks multiply.

Planned work:

- Introduce a pipeline engine with explicit stages.
- Add parallel execution where build artifacts allow it.
- Add incremental result caching.
- Define analyzer plugin interfaces.
- Define report format versioning.
- Add schema tests for JSON output.
- Add stable internal domain models for diagnostics, tests, coverage, and artifacts.
- Add compatibility tests across supported Clang and LLVM versions.

Exit criteria:

- Adding a new analyzer does not require changing unrelated pipeline code.
- Large repositories avoid repeating unnecessary work.
- Report consumers can rely on versioned schemas.

## Phase 6: Developer Experience and Distribution

Goal: make CppGauntlet easy to install, use, and adopt in teams.

Planned work:

- Publish binaries for macOS and Linux.
- Publish to crates.io.
- Add Homebrew tap support.
- Add shell completions.
- Add GitHub Actions examples.
- Add pre-commit examples.
- Add starter templates for common C++ project layouts.
- Add documentation site.
- Add comparison guides for `clang-tidy`, sanitizers, `ctest`, and coverage tooling.

Exit criteria:

- A new user can install and run CppGauntlet in under five minutes.
- CI setup is copy-pasteable for common workflows.
- Documentation explains both quick start and advanced configuration.

## Phase 7: Team and Platform Features

Goal: prepare for product-level scale beyond a local CLI.

Planned work:

- Add SARIF output for code scanning platforms.
- Add GitHub Checks annotation support.
- Add historical report ingestion format.
- Add optional hosted dashboard design.
- Add repository health views.
- Add pull request regression views.
- Add organization-level policy configuration.
- Add signed release artifacts.
- Add supply-chain security checks for distributed binaries.

Exit criteria:

- CppGauntlet can fit into enterprise CI and code review workflows.
- Local CLI results and hosted results use the same core report schema.
- Security-conscious teams can verify release artifacts.

## Long-Term Product Bets

- **C++ quality cockpit:** one place to understand compiler diagnostics, static analysis, sanitizer failures, tests, coverage, and fuzzing.
- **Regression-first workflows:** focus attention on new risk introduced by a change.
- **Compiler-aware intelligence:** use Clang and LLVM data structures rather than fragile text scraping where possible.
- **Build-system empathy:** integrate with existing build systems instead of forcing users to rebuild their projects around CppGauntlet.
- **Open core:** keep local verification open and useful, while optional team-scale services can build on top.

## Near-Term Priorities

1. Add GitHub Actions examples for diff-based changed-line coverage.
2. Add project-level fuzz target discovery.
3. Add contributor workflow automation for issue labels and PR checks.
4. Add automated release builds for macOS and Linux.
5. Add PR comment summaries for baseline update artifacts.
6. Add explicit report schema migration notes for future breaking changes.

## Non-Goals

- CppGauntlet will not try to prove full program correctness.
- CppGauntlet will not replace Clang, CMake, `clang-tidy`, sanitizers, or test frameworks.
- CppGauntlet will not require a hosted service for core functionality.
- CppGauntlet will not hide the underlying commands it runs.

## Success Metrics

- Time from install to first useful report.
- Number of supported C++ project layouts.
- Stability of JSON report schemas.
- CI adoption success rate.
- Reduction in repeated diagnostic noise.
- Percentage of issues classified as new, existing, or fixed.
- Contributor time needed to add a new analyzer.
