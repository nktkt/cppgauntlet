# Contributor Automation

CppGauntlet uses GitHub Actions to keep issue triage and pull request metadata consistent without requiring contributors to remember repository-specific labels.

See [LABEL_TAXONOMY.md](LABEL_TAXONOMY.md) for the full support stage, priority, work type, and product area label taxonomy.

## Workflow

The automation lives in [.github/workflows/contributor-automation.yml](../.github/workflows/contributor-automation.yml).

It runs on:

- `issues`: `opened`, `edited`, and `reopened`
- `pull_request_target`: `opened`, `edited`, `reopened`, `synchronize`, and `ready_for_review`

The `pull_request_target` job does not checkout or execute pull request code. It only reads pull request metadata through the GitHub API.

## Issue Labels

Issue automation always applies `needs-triage`. If the issue has no priority label, it also applies `priority: needs-priority`. It then adds area labels from the issue title and body, including:

- `area: baseline`
- `area: build-systems`
- `area: ci`
- `area: cli`
- `area: configuration`
- `area: coverage`
- `area: docs`
- `area: fuzzing`
- `area: reports`
- `area: static-analysis`

Missing labels are created automatically with stable colors and descriptions.

Priority escalation remains a maintainer decision. Use [LABEL_TAXONOMY.md](LABEL_TAXONOMY.md) to decide when to move an issue to `priority: critical`, `priority: high`, `priority: medium`, or `priority: low`.

The workflow also creates managed release labels such as `release: breaking`, `release: highlight`, and `release: skip`. Maintainers apply those labels manually when a pull request needs special placement or exclusion in generated release notes.

## Pull Request Labels

Pull request automation applies `needs-review` or `status: draft`, then labels the pull request from changed files:

- `.github/**` and `examples/github-actions/**` -> `area: ci`
- `docs/**`, `README.md`, and `ROADMAP.md` -> `area: docs`
- `tests/**` -> `area: tests`
- `src/**` -> `area: core`
- report, schema, and SARIF paths -> `area: reports`
- coverage paths -> `area: coverage`
- fuzzing paths -> `area: fuzzing`

The taxonomy document is the source of truth for label meaning. The workflow is the executable mapping from issue text and pull request paths to that taxonomy.

## Pull Request Body Check

The PR body check requires:

- `## Summary`
- `## Validation`
- `## Compatibility`

The summary section must contain filled-in content. The validation section must either include at least one checked item or explicitly explain skipped, unavailable, or not-run validation. The compatibility section must include filled-in content or at least one checked item.

This check is intentionally lightweight. It verifies that review context exists, while Rust formatting, linting, and tests remain enforced by the normal CI workflow.
