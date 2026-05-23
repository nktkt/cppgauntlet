# Label Taxonomy

CppGauntlet labels are grouped by support stage, work type, and product area. Contributor automation creates missing managed labels with stable colors and descriptions.

## Support Stage Labels

Use support stage labels to show where an issue or pull request is in the maintainer workflow.

| Label | Used on | Meaning |
| --- | --- | --- |
| `needs-triage` | issues | A maintainer has not classified priority, area, or next action yet. |
| `needs-review` | pull requests | The pull request is ready for maintainer review. |
| `status: draft` | pull requests | The pull request is open but not ready for review. |

Only one pull request stage label should be active at a time: `needs-review` or `status: draft`.

## Work Type Labels

Issue templates apply these labels before automation adds area labels.

| Label | Used on | Meaning |
| --- | --- | --- |
| `bug` | issues | Broken, surprising, or incorrect CppGauntlet behavior. |
| `enhancement` | issues | New workflow, integration, analyzer, output format, or product improvement. |

## Priority Labels

Priority labels are assigned during maintainer triage. Issue automation applies `priority: needs-priority` when an issue has no existing priority label.

| Label | Meaning | Escalation trigger |
| --- | --- | --- |
| `priority: needs-priority` | The issue needs a maintainer priority decision. | Applied automatically when no priority exists. |
| `priority: critical` | Release-blocking, security-sensitive, or data-loss risk. | Active users cannot safely install, run, or verify releases. |
| `priority: high` | Important regression or blocked core workflow. | CI adoption, release automation, report compatibility, or primary checks are blocked. |
| `priority: medium` | Useful product work with no immediate release risk. | Improves a supported workflow but has a workaround. |
| `priority: low` | Opportunistic cleanup, polish, or exploratory work. | Nice to have, unclear urgency, or low user impact. |

Only one priority label should be active at a time.

## Product Area Labels

Use area labels to route work to the relevant subsystem.

| Label | Scope |
| --- | --- |
| `area: baseline` | Diagnostic baselines, baseline updates, and new versus existing issue classification. |
| `area: build-systems` | CMake, `compile_commands.json`, build discovery, and generated build artifacts. |
| `area: ci` | GitHub Actions, Code Scanning, release workflows, and CI examples. |
| `area: cli` | Command names, arguments, exit codes, and terminal output. |
| `area: configuration` | `cppgauntlet.yaml`, config loading, validation, and CLI override behavior. |
| `area: core` | Rust implementation shared across check execution, pipelines, and domain models. |
| `area: coverage` | LLVM coverage, changed-line coverage, coverage gates, and coverage artifacts. |
| `area: docs` | README, roadmap, guides, examples, and contributor-facing documentation. |
| `area: fuzzing` | libFuzzer discovery, compilation, execution, corpora, and crash artifacts. |
| `area: reports` | JSON, Markdown, HTML, SARIF, report schema, diagnostics, and fingerprints. |
| `area: static-analysis` | `clang-tidy`, Clang Static Analyzer, analyzer severities, and analyzer policies. |
| `area: tests` | Rust tests, fixtures, integration coverage, and test infrastructure. |

## Automation Rules

Issue automation applies `needs-triage`, `priority: needs-priority` when no priority exists, and area labels from the issue title and body.

Pull request automation applies `needs-review` or `status: draft`, then maps changed files to area labels:

| Paths or terms | Label |
| --- | --- |
| `.github/**`, `examples/github-actions/**` | `area: ci` |
| `docs/**`, `README.md`, `ROADMAP.md` | `area: docs` |
| `tests/**` | `area: tests` |
| `src/**` | `area: core` |
| report, schema, or SARIF paths | `area: reports` |
| coverage paths | `area: coverage` |
| fuzzing paths | `area: fuzzing` |

When a change spans multiple subsystems, keep every accurate area label. Do not remove a broad area label just because a more specific area label is also present.

## Escalation Guidance

Escalate an issue when new information shows broader risk than the current priority suggests:

- move to `priority: critical` for release-blocking failures, security-sensitive distribution problems, or unsafe output that could mislead CI users
- move to `priority: high` when a supported workflow is blocked without a practical workaround
- move to `priority: medium` when the issue affects adoption but an explicit workaround exists
- move to `priority: low` when the issue is polish, cleanup, or investigation

When escalating, leave a short comment with the concrete trigger, such as failing release attestation verification, schema compatibility regression, or blocked CMake coverage adoption.

## Naming Rules

- Use lower-case labels.
- Use `area: <name>` for product areas.
- Use `priority: <name>` for maintainer priority.
- Use `status: <name>` for workflow state.
- Keep unscoped labels only for common GitHub issue types such as `bug` and `enhancement`.
- Prefer adding a documented label to inventing an ad hoc one in a single issue.
