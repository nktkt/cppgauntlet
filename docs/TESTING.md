# Testing

CppGauntlet can run tests through CTest or a custom shell command.

## CTest

For CMake projects:

```bash
cppgauntlet check ./my-cmake-project --ctest
```

This builds the configured CMake project and runs:

```bash
ctest --test-dir <artifact_dir>/cmake-build --output-on-failure
```

The report records:

- `cmake_build`
- `ctest`

## Custom Commands

Use `--test-command` for projects that do not use CTest or need an extra validation step:

```bash
cppgauntlet check ./project --test-command "make test"
cppgauntlet check ./project/compile_commands.json --test-command "./scripts/test.sh"
cppgauntlet check main.cpp --test-command "test -f expected-output.txt"
```

The custom command runs after compile and analyzer stages have passed. For source-file checks it runs from the source file directory. For `compile_commands.json` checks it runs from the compilation database directory. For CMake projects it runs from the project directory.

When coverage is enabled for a raw `compile_commands.json` target, the command runs as `coverage_test_command` with `LLVM_PROFILE_FILE` set so profile data can be collected.

The stage name is:

```text
test_command
```

If an earlier stage fails, `test_command` is skipped. If the command exits non-zero or times out, the report fails.

## Configuration

```yaml
test:
  ctest: false
  command: "make test"
```

CLI arguments override configuration:

```bash
cppgauntlet check ./project --test-command "pytest tests/cpp"
```
