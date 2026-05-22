# clang-tidy

CppGauntlet can run `clang-tidy` as an optional static-analysis stage.

For a single source file:

```bash
cppgauntlet check main.cpp --clang-tidy
```

For a project with `compile_commands.json`:

```bash
cppgauntlet check ./build --clang-tidy
cppgauntlet check ./compile_commands.json --clang-tidy
```

For a CMake project, CppGauntlet first configures CMake with `CMAKE_EXPORT_COMPILE_COMMANDS=ON`, then runs `clang-tidy` against the generated compilation database.

## Options

```bash
cppgauntlet check main.cpp \
  --clang-tidy \
  --clang-tidy-bin clang-tidy \
  --clang-tidy-checks "bugprone-*,modernize-*"
```

- `--clang-tidy`: enable the static-analysis stage
- `--clang-tidy-bin`: override the executable path
- `--clang-tidy-checks`: pass a checks expression as `--checks=<value>`

Passing `--clang-tidy-bin` or `--clang-tidy-checks` also enables the stage.

## Configuration

```yaml
static_analysis:
  clang_tidy: true
  clang_tidy_bin: clang-tidy
  clang_tidy_checks: "bugprone-*,modernize-*"
```

Single-file checks run:

```bash
clang-tidy <source> -- <standard>
```

Project checks run:

```bash
clang-tidy <source> -p <compile_commands_directory>
```

If compile checks fail, the matching `clang_tidy` stages are skipped. `clang-tidy` diagnostics are parsed from both stdout and stderr and included in the JSON report.
