# Release Checklist

CppGauntlet has automated GitHub release builds for macOS and Linux. The release workflow builds release binaries, packages archives, writes SHA-256 checksums, creates signed GitHub artifact attestations, generates release notes from merged pull requests, uploads workflow artifacts, and attaches assets to tagged GitHub releases.

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
4. Confirm the generated macOS and Linux archives, `.sha256` files, and `.intoto.jsonl` attestation bundles are attached to the GitHub release.
5. Review the generated release notes and add installation commands or compatibility notes when needed.

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

## Generated Release Notes

Tag builds call GitHub's generated release notes API before creating the release:

```bash
gh api "repos/${GITHUB_REPOSITORY}/releases/generate-notes" \
  -X POST \
  -f tag_name="${GITHUB_REF_NAME}" \
  -f target_commitish="${GITHUB_SHA}" \
  --jq '.body'
```

The generated body is written to:

```text
cppgauntlet-<version>-<platform>-<arch>-release-notes.md
```

The release creation step passes that file to:

```bash
gh release create "$tag" --title "$tag" --notes-file <release-notes.md>
```

Manual `workflow_dispatch` builds write a short manual-build note instead of creating a GitHub release.

## Artifact Signing

Release archives and checksum files are signed with GitHub artifact attestations through `actions/attest-build-provenance`.

The release workflow grants:

```yaml
permissions:
  contents: write
  id-token: write
  attestations: write
```

For each platform archive, the workflow attests:

- `cppgauntlet-<version>-<platform>-<arch>.tar.gz`
- `cppgauntlet-<version>-<platform>-<arch>.tar.gz.sha256`

It also copies the generated attestation bundle to:

```text
cppgauntlet-<version>-<platform>-<arch>.intoto.jsonl
```

Verify a downloaded archive with:

```bash
shasum -a 256 -c cppgauntlet-v0.1.0-linux-x86_64.tar.gz.sha256
gh attestation verify cppgauntlet-v0.1.0-linux-x86_64.tar.gz \
  --repo nktkt/cppgauntlet \
  --signer-workflow nktkt/cppgauntlet/.github/workflows/release.yml
```

To verify from a downloaded bundle instead of fetching attestations from GitHub:

```bash
gh attestation verify cppgauntlet-v0.1.0-linux-x86_64.tar.gz \
  --repo nktkt/cppgauntlet \
  --bundle cppgauntlet-v0.1.0-linux-x86_64.intoto.jsonl \
  --signer-workflow nktkt/cppgauntlet/.github/workflows/release.yml
```

## Future Automation

Planned follow-up automation:

- crates.io publication
- Homebrew formula update
- release SBOM generation
- release notes policy configuration
