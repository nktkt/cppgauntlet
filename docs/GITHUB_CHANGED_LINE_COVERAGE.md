# GitHub Changed-Line Coverage

CppGauntlet can fail pull requests when newly changed coverable lines fall below a configured LLVM coverage threshold. The GitHub Actions workflow writes a zero-context unified diff, passes it to `--changed-lines-diff`, and uploads the report artifacts for review.

## Workflow Example

Copy [examples/github-actions/changed-line-coverage.yml](../examples/github-actions/changed-line-coverage.yml) into `.github/workflows/cppgauntlet-changed-line-coverage.yml`, then update `CPPGAUNTLET_TARGET` and `CPPGAUNTLET_MIN_CHANGED_LINE_COVERAGE`.

The example workflow:

- checks out full history with `fetch-depth: 0`
- writes `.cppgauntlet/changed.diff` from the pull request base/head range or push range
- runs `cppgauntlet check` with `--coverage`, `--changed-lines-diff`, and `--min-changed-line-coverage`
- optionally passes `--test-command` when `CPPGAUNTLET_TEST_COMMAND` is set
- writes `.cppgauntlet/cppgauntlet-report.json`
- writes `.cppgauntlet/cppgauntlet-report.md`
- uploads the diff, reports, and coverage artifacts with `actions/upload-artifact@v4`
- fails the job after artifact upload when the coverage gate fails

## Target Modes

For a single source file, set:

```yaml
CPPGAUNTLET_TARGET: src/main.cpp
CPPGAUNTLET_TEST_COMMAND: ""
```

For a raw `compile_commands.json` project, point at the file or containing directory and provide a command that runs the instrumented tests:

```yaml
CPPGAUNTLET_TARGET: compile_commands.json
CPPGAUNTLET_TEST_COMMAND: ./scripts/test.sh
```

For a CMake project with CTest, point at the project directory:

```yaml
CPPGAUNTLET_TARGET: .
CPPGAUNTLET_TEST_COMMAND: ""
```

CppGauntlet automatically uses CTest for CMake coverage mode.

## Minimal Commands

```bash
mkdir -p .cppgauntlet
git diff -U0 origin/main...HEAD > .cppgauntlet/changed.diff

cppgauntlet check . \
  --coverage \
  --changed-lines-diff .cppgauntlet/changed.diff \
  --min-changed-line-coverage 80 \
  --report .cppgauntlet/cppgauntlet-report.json \
  --markdown-report .cppgauntlet/cppgauntlet-report.md
```

Use `--coverage-source` and `--coverage-object` when the default LLVM coverage discovery should be narrowed to specific source files or binaries.

## Reviewing Failures

Download the `cppgauntlet-changed-line-coverage` artifact from the Actions run. Inspect:

- `changed.diff` for the lines CppGauntlet considered new
- `cppgauntlet-report.md` for a human-readable summary
- `cppgauntlet-report.json` for `summary.coverage.changed_lines`
- `.cppgauntlet/coverage/**` for raw LLVM coverage exports

Deleted-only hunks do not affect changed-line coverage because they have no coverable line in the current source tree.
