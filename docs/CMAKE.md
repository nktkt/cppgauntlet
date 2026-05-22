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

Current CMake support configures the project and validates translation units with syntax-only compile checks.

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

Project-level sanitizer builds and coverage are not implemented yet. Those workflows will build on this generated compilation database and CTest support.
