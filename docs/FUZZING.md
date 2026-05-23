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

Project checks suffix per-target stages with the source path, for example `fuzz_compile:src/parser_fuzz.cpp` and `fuzz_run:src/parser_fuzz.cpp`.

Crash artifacts are written under `.cppgauntlet/fuzz/artifacts` by passing libFuzzer's `-artifact_prefix` option.

When no corpus is supplied, single-source checks use `.cppgauntlet/fuzz/corpus`. Project checks create a separate default corpus directory per discovered fuzz target under `.cppgauntlet/fuzz/corpus/`.

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
