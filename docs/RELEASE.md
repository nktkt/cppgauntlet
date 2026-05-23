# Release Checklist

CppGauntlet has automated GitHub release builds for macOS and Linux. The release workflow builds release binaries, packages archives, writes SHA-256 checksums, uploads workflow artifacts, and attaches assets to tagged GitHub releases.

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
2. Push a tag matching `v*`, for example `v0.1.0`.
3. Wait for the `Release` and `CI` workflows to pass on the tag.
4. Confirm the generated macOS and Linux archives and `.sha256` files are attached to the GitHub release.
5. Include installation commands and a short compatibility note in the release notes.

## Automated Binary Builds

The release workflow lives in [.github/workflows/release.yml](../.github/workflows/release.yml).

It runs on:

- tags matching `v*`
- manual `workflow_dispatch`

For each platform, the workflow runs:

```bash
cargo test --locked
cargo build --release --locked
```

It packages:

- `cppgauntlet`
- `README.md`
- `LICENSE`
- `docs/INSTALLATION.md`
- `docs/RELEASE.md`

Archive names use:

```text
cppgauntlet-<version>-<platform>-<arch>.tar.gz
```

Each archive is paired with a `.sha256` checksum file. Tag builds upload assets to the GitHub release with `gh release upload --clobber`.

## Future Automation

Planned follow-up automation:

- signed release artifacts
- crates.io publication
- Homebrew formula update
