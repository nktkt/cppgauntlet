# Release Checklist

CppGauntlet releases are not yet automated. This checklist records the expected package and artifact checks before the first public release.

## Version Metadata

1. Update `Cargo.toml` `version`.
2. Confirm `README.md`, `LICENSE`, `CONTRIBUTING.md`, and `CODE_OF_CONDUCT.md` are included in package metadata.
3. Confirm `docs/INSTALLATION.md` reflects the current install channels.
4. Confirm `ROADMAP.md` near-term priorities match the release scope.

## Local Validation

Run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo package --list
cargo package --no-verify
```

Use `--offline` for local package checks when the crates.io index is already cached and network access is unavailable.

Inspect the package file list and make sure it includes:

- `Cargo.toml`
- `Cargo.lock`
- `LICENSE`
- `README.md`
- `docs/`
- `examples/`
- `src/`
- `tests/`

## GitHub Release Preparation

1. Tag the release from a clean `main` branch.
2. Wait for GitHub Actions to pass on the tag.
3. Attach generated archives or binaries when binary packaging is available.
4. Include installation commands and a short compatibility note in the release notes.

## Future Automation

Planned release automation:

- cross-platform release builds for macOS and Linux
- checksums for binary artifacts
- signed release artifacts
- crates.io publication
- Homebrew formula update
