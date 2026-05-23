# GitHub Release Download Verification

Use [release-download-verification.yml](../examples/github-actions/release-download-verification.yml) when a repository should install CppGauntlet from GitHub Releases and verify the downloaded archive before use.

The workflow is intended for downstream CI, release promotion checks, and scheduled install smoke tests. It runs on Linux and macOS so the asset name matches the runner platform and architecture.

The workflow:

- resolves the latest CppGauntlet release, or uses the manual `CPPGAUNTLET_VERSION` input
- derives the expected `cppgauntlet-<version>-<platform>-<arch>.tar.gz` asset name
- downloads the archive and matching `.sha256` file with `gh release download`
- verifies the checksum with `shasum -a 256 -c`
- extracts the archive
- runs `cppgauntlet --version` and `cppgauntlet --help`

## Manual Version Selection

Use the `workflow_dispatch` input to verify a specific tag:

```yaml
version: v0.1.0
```

When the input is empty, the workflow resolves the current latest release with:

```bash
gh release view --repo "$CPPGAUNTLET_REPOSITORY" --json tagName --jq '.tagName'
```

## Asset Naming

Release assets are expected to follow the release workflow naming convention:

```text
cppgauntlet-<version>-<platform>-<arch>.tar.gz
cppgauntlet-<version>-<platform>-<arch>.tar.gz.sha256
```

The example computes `<arch>` with `uname -m`, matching the release workflow's archive packaging step.

## Provenance

Checksum verification proves the downloaded bytes match the published checksum file. For release origin and workflow identity checks, also verify GitHub artifact attestations as described in [RELEASE.md](RELEASE.md#artifact-signing).
