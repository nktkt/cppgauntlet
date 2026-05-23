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

Issue automation applies `needs-triage` and area labels from the issue title and body.

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

## Naming Rules

- Use lower-case labels.
- Use `area: <name>` for product areas.
- Use `status: <name>` for workflow state.
- Keep unscoped labels only for common GitHub issue types such as `bug` and `enhancement`.
- Prefer adding a documented label to inventing an ad hoc one in a single issue.
