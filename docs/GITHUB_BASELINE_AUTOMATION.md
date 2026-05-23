# GitHub Baseline Automation

CppGauntlet can keep known diagnostic debt in a committed baseline while making new diagnostics fail CI. The review workflow uploads a baseline candidate and a change summary so a pull request can decide whether to fix diagnostics or intentionally update the baseline.

## Workflow Example

Copy [examples/github-actions/baseline-review.yml](../examples/github-actions/baseline-review.yml) into `.github/workflows/cppgauntlet-baseline-review.yml`, then update `CPPGAUNTLET_TARGET` and `CPPGAUNTLET_BASELINE`.

The example workflow:

- installs CppGauntlet with `cargo install --git`
- requires a committed baseline at `.cppgauntlet/baseline.json`
- runs `cppgauntlet check` with `--baseline` and `--fail-on-new-diagnostics`
- keeps running after the check step fails so review artifacts can be produced
- writes `.cppgauntlet/baseline.candidate.json`
- writes `.cppgauntlet/baseline-update.md`
- uploads the report, candidate baseline, and update summary with `actions/upload-artifact@v4`
- posts or updates a sticky pull request comment with the baseline update summary
- fails the job after artifact upload when new diagnostics were found

## Initial Baseline

Create and review the first baseline locally:

```bash
cppgauntlet check path/to/main.cpp --report .cppgauntlet/cppgauntlet-report.json
cppgauntlet baseline update \
  --report .cppgauntlet/cppgauntlet-report.json \
  --output .cppgauntlet/baseline.json
```

Commit `.cppgauntlet/baseline.json` after reviewing the known diagnostics it contains.

## Review Flow

When CI fails on new diagnostics:

1. Read the `CppGauntlet Baseline Review` pull request comment for new, resolved, and unchanged diagnostic counts.
2. Download the `cppgauntlet-baseline-review` artifact from the GitHub Actions run.
3. Inspect `cppgauntlet-report.md` to see the current diagnostics.
4. If the new diagnostics should be accepted as known debt, replace the committed baseline with `baseline.candidate.json`.
5. Prefer fixing new diagnostics instead of updating the baseline when possible.

The PR comment step uses `issues: write` and `pull-requests: read` permissions. It is marked `continue-on-error` so permission limits on forked pull requests do not hide the underlying CppGauntlet result.

## Minimal Commands

The core pattern is:

```bash
cppgauntlet check path/to/main.cpp \
  --baseline .cppgauntlet/baseline.json \
  --fail-on-new-diagnostics \
  --report .cppgauntlet/cppgauntlet-report.json

cppgauntlet --format markdown baseline update \
  --report .cppgauntlet/cppgauntlet-report.json \
  --previous .cppgauntlet/baseline.json \
  --output .cppgauntlet/baseline.candidate.json \
  > .cppgauntlet/baseline-update.md
```

Use `--format json` instead of `--format markdown` when another CI step should consume the baseline update summary.
