# Contributing to CppGauntlet

CppGauntlet is an early Rust CLI for C++ verification workflows. Contributions should keep the tool local-first, transparent, and useful in CI.

## Development Setup

Install the stable Rust toolchain and the C++ tools needed by the feature you are working on. The core test suite can run without every optional external tool, but integration tests will use tools such as `clang++`, CMake, CTest, `clang-tidy`, `llvm-cov`, and `llvm-profdata` when they are available.

```bash
rustup toolchain install stable
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Local Workflow

1. Open or pick an issue before starting broad work.
2. Keep changes focused on one product behavior or documentation surface.
3. Prefer existing modules and report schema patterns over new abstractions.
4. Add or update integration tests for user-visible CLI behavior.
5. Update documentation when commands, configuration, report schema, or CI behavior changes.

## Pull Requests

Pull requests should include:

- a concise summary of the behavior change
- the local validation commands that passed
- notes about any skipped validation or unavailable external tools
- report schema or configuration compatibility notes when relevant

Before opening a PR, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Issue Guidelines

Use the bug report template for broken behavior and the feature request template for product or workflow proposals.

Good issues include:

- operating system and tool versions
- the exact `cppgauntlet` command or configuration
- expected behavior
- actual output, report snippets, or CI logs
- a small reproducible C++ fixture when possible

## Report Schema Changes

Report JSON is a public contract. Changes that add fields should be documented in `docs/REPORT_SCHEMA.md` and covered by tests. Changes that remove or rename fields should include a compatibility plan.

## Code of Conduct

All contributors are expected to follow the [Code of Conduct](CODE_OF_CONDUCT.md).
