# Doctor

`cppgauntlet doctor` checks whether external tools needed by CppGauntlet workflows are available.

```bash
cppgauntlet doctor
```

The default required tool is:

- `clang++`

The default optional tools are:

- `clang-tidy`
- `llvm-cov`
- `llvm-profdata`
- `cmake`
- `ctest`
- `ninja`

Missing optional tools are reported but do not fail the command. Missing required tools make the command exit with status code `1`.

`llvm-cov` and `llvm-profdata` are used by `cppgauntlet check <file> --coverage`.

## JSON Output

```bash
cppgauntlet --format json doctor
```

The JSON report includes:

- schema version
- overall status
- missing required tools
- one record per checked tool
- command used for the version check
- detected version line or error detail

## Custom Tool Sets

Passing `--required-tool` or `--optional-tool` replaces the default tool set.

```bash
cppgauntlet doctor --required-tool clang++ --optional-tool cmake
```

Repeat either flag to check multiple tools.
