# GitHub Code Scanning

CppGauntlet can generate SARIF and upload it to GitHub Code Scanning from GitHub Actions.

## Workflow Example

Copy [examples/github-actions/code-scanning.yml](../examples/github-actions/code-scanning.yml) into `.github/workflows/cppgauntlet-code-scanning.yml`, then replace `path/to/main.cpp` with the file or project target you want CppGauntlet to check.

The example workflow:

- installs CppGauntlet with `cargo install --git`
- writes `.cppgauntlet/cppgauntlet.sarif.json`
- uploads it with `github/codeql-action/upload-sarif@v4`
- uses `security-events: write`, `actions: read`, and `contents: read` permissions
- keeps the SARIF upload step reachable with `continue-on-error: true`

## Minimal Upload Step

```yaml
- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v4
  with:
    sarif_file: .cppgauntlet/cppgauntlet.sarif.json
    category: cppgauntlet
```

## Notes

GitHub Code Scanning accepts SARIF 2.1.0. For private and internal repositories, GitHub Code Security must be enabled.

If a SARIF file has no `partialFingerprints`, GitHub's upload action can calculate them when the repository contains both the SARIF file and the analyzed source code.
