# CMake

CppGauntlet can check a CMake project when the target directory contains `CMakeLists.txt`.

```bash
cppgauntlet check ./my-cmake-project
```

When no `compile_commands.json` is already present, CppGauntlet runs:

```bash
cmake -S <project> -B <artifact_dir>/cmake-build -DCMAKE_EXPORT_COMPILE_COMMANDS=ON
```

The generated compilation database is then checked with the same project pipeline described in [COMPILATION_DATABASE.md](COMPILATION_DATABASE.md).

Static analysis can be enabled for the generated compilation database:

```bash
cppgauntlet check ./my-cmake-project --clang-tidy
```

## Artifacts

By default, CMake files are generated under:

```text
.cppgauntlet/cmake-build
```

The JSON report is still written to:

```text
.cppgauntlet/cppgauntlet-report.json
```

## Current Scope

Current CMake support configures the project, validates translation units with syntax-only compile checks, can run `clang-tidy`, and can optionally build and run CTest.

## CTest

Use `--ctest` to build the configured project and run CTest:

```bash
cppgauntlet check ./my-cmake-project --ctest
```

This adds two stages after syntax-only compile checks:

- `cmake_build`
- `ctest`

CppGauntlet runs:

```bash
cmake --build <artifact_dir>/cmake-build
ctest --test-dir <artifact_dir>/cmake-build --output-on-failure
```

If syntax-only compile checks fail, `cmake_build` and `ctest` are skipped.

Custom test commands can also be used with CMake projects:

```bash
cppgauntlet check ./my-cmake-project --test-command "ctest --test-dir .cppgauntlet/cmake-build"
```

The command runs from the project directory after compile, analyzer, and optional CTest stages have passed.

Project-level sanitizer builds are not implemented yet. That workflow will build on this generated compilation database and CTest support.

## Coverage

Use `--coverage` to configure a separate coverage-instrumented build, build it, run CTest with `LLVM_PROFILE_FILE`, and collect an LLVM coverage summary:

```bash
cppgauntlet check ./my-cmake-project --coverage
```

This adds:

- `coverage_cmake_configure`
- `coverage_cmake_build`
- `coverage_ctest`
- `coverage_merge`
- `coverage_report`

Coverage artifacts are written under:

```text
.cppgauntlet/cmake-coverage-build
.cppgauntlet/coverage/cmake
```

## GitHub Actions

Use [examples/github-actions/cmake-coverage.yml](../examples/github-actions/cmake-coverage.yml) as a reusable CI starting point for CMake coverage checks. It runs `cppgauntlet check . --coverage`, applies a configurable `--min-line-coverage` gate, uploads report artifacts, and keeps the generated CMake coverage build for inspection.
