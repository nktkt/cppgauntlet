# Fuzzing

CppGauntlet can run a short libFuzzer smoke workflow for a single C++ fuzz target.

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

Pass one or more corpus directories with `--fuzz-corpus`:

```bash
cppgauntlet check fuzz_target.cpp --fuzz --fuzz-corpus tests/corpus --fuzz-seconds 10
```

When no corpus is supplied, CppGauntlet creates and uses `.cppgauntlet/fuzz/corpus`.

## Stages

The fuzz workflow records:

- `fuzz_compile`: builds the libFuzzer executable
- `fuzz_run`: runs the executable with `-max_total_time=<seconds>`

Crash artifacts are written under `.cppgauntlet/fuzz/artifacts` by passing libFuzzer's `-artifact_prefix` option.

`--fuzz` is currently supported for single source files. Project-level fuzz target discovery and coverage-guided fuzzing reports are planned future work.
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
