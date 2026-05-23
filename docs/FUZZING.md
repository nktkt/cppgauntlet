# Fuzzing

CppGauntlet can run short libFuzzer smoke workflows for a single C++ fuzz target or for fuzz targets discovered from a project compilation database.

## Usage

Create a source file that exports `LLVMFuzzerTestOneInput`:

```cpp
#include <cstddef>
#include <cstdint>

extern "C" int LLVMFuzzerTestOneInput(const std::uint8_t *data, std::size_t size) {
    return size > 0 && data[0] == 0xff ? 0 : 0;
}
```

Run a smoke fuzzing pass:

```bash
cppgauntlet check fuzz_target.cpp --fuzz --fuzz-seconds 5
```

The source is compiled with `-fsanitize=fuzzer` plus any configured sanitizer set. With the default sanitizer configuration, the fuzz target is built with libFuzzer, AddressSanitizer, and UndefinedBehaviorSanitizer.

For projects, point CppGauntlet at a directory containing `compile_commands.json`, a `compile_commands.json` file, or a CMake project:

```bash
cppgauntlet check . --fuzz --fuzz-seconds 5
cppgauntlet check ./compile_commands.json --fuzz --fuzz-seconds 5
cppgauntlet check ./my-cmake-project --fuzz --fuzz-seconds 5
```

Project fuzzing scans compilation database entries for sources containing `LLVMFuzzerTestOneInput`. Each discovered target is compiled with its compilation database command rewritten into a libFuzzer executable, preserving include paths and preprocessor definitions while replacing compile-only output flags.

Pass one or more corpus directories with `--fuzz-corpus`:

```bash
cppgauntlet check fuzz_target.cpp --fuzz --fuzz-corpus tests/corpus --fuzz-seconds 10
```

When no corpus is supplied, CppGauntlet creates and uses `.cppgauntlet/fuzz/corpus`.

## Stages

The fuzz workflow records:

- `fuzz_discover`: for project checks, lists discovered fuzz targets
- `fuzz_compile`: builds the libFuzzer executable
- `fuzz_run`: runs the executable with `-max_total_time=<seconds>`
- `fuzz_summary:<source path>`: for project checks, writes a per-target artifact summary

Project checks suffix per-target stages with the source path, for example `fuzz_compile:src/parser_fuzz.cpp`, `fuzz_run:src/parser_fuzz.cpp`, and `fuzz_summary:src/parser_fuzz.cpp`.

Crash artifacts are written under `.cppgauntlet/fuzz/artifacts` by passing libFuzzer's `-artifact_prefix` option.

For project checks, CppGauntlet creates one crash artifact directory per discovered fuzz target under `.cppgauntlet/fuzz/artifacts/<target-id>/`.

Project fuzz summaries are written under `.cppgauntlet/fuzz/summaries/<target-id>.json`. Each summary records the source path, target artifact id, fuzz executable, corpus paths, fuzz duration, crash artifact directory, discovered crash artifact files, compile stage status, and run stage status.

When no corpus is supplied, single-source checks use `.cppgauntlet/fuzz/corpus`. Project checks create a separate default corpus directory per discovered fuzz target under `.cppgauntlet/fuzz/corpus/`.

## GitHub Actions

Use [examples/github-actions/fuzz-crash-artifacts.yml](../examples/github-actions/fuzz-crash-artifacts.yml) to run fuzz smoke checks in CI and upload crash artifacts for review.

The workflow:

- optionally runs `CPPGAUNTLET_CONFIGURE_COMMAND` before fuzzing
- runs `cppgauntlet check "$CPPGAUNTLET_TARGET" --fuzz`
- keeps going after the fuzz step fails so artifacts can be uploaded
- uploads JSON and Markdown reports
- uploads `.cppgauntlet/fuzz/artifacts/**`
- uploads `.cppgauntlet/fuzz/summaries/**`
- fails the job after upload when CppGauntlet found a fuzz failure

Download the `cppgauntlet-fuzz-artifacts` workflow artifact to inspect libFuzzer crash files and per-target summary JSON files.

Use [examples/github-actions/fuzz-corpus-retention.yml](../examples/github-actions/fuzz-corpus-retention.yml) for scheduled or manually triggered long-running fuzz jobs that should reuse corpus inputs across runs.

The retention workflow:

- runs on `workflow_dispatch` and a nightly `schedule`
- restores `.cppgauntlet/fuzz/corpus` with `actions/cache/restore@v5`
- runs a longer fuzz pass with `CPPGAUNTLET_FUZZ_SECONDS`
- saves the updated corpus with `actions/cache/save@v5` when the fuzz gate passes
- uploads diagnostics, crash artifacts, summaries, and the retained corpus with `retention-days: 30`
- fails the job after uploads when CppGauntlet finds a fuzz failure

Keep the default corpus path when possible. For project fuzzing, CppGauntlet creates per-target corpus directories under `.cppgauntlet/fuzz/corpus/`, which makes the cache reusable across scheduled runs.

Coverage-guided fuzzing trend reports are planned future work.
`--coverage` is not supported together with `--fuzz` yet.

## Configuration

```yaml
fuzz:
  enabled: true
  seconds: 5
  corpus:
    - tests/corpus
```

CLI arguments override configuration:

```bash
cppgauntlet check fuzz_target.cpp --fuzz --fuzz-seconds 2
```
