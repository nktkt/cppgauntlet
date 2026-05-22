# Compilation Database

CppGauntlet can check a project through `compile_commands.json`.

```bash
cppgauntlet check ./build
cppgauntlet check ./compile_commands.json
```

When the target is a directory, CppGauntlet looks for:

1. `<target>/compile_commands.json`
2. `<target>/build/compile_commands.json`

The current implementation runs one syntax-only compile stage per compilation database entry. It appends `-fsyntax-only` to each recorded command so the check validates the translation unit without producing object files.

When `--clang-tidy` is enabled, CppGauntlet runs one `clang-tidy:<source path>` stage per compilation database entry after all syntax-only compile stages pass:

```bash
cppgauntlet check ./build --clang-tidy
cppgauntlet check ./compile_commands.json --clang-tidy --clang-tidy-checks "bugprone-*,modernize-*"
```

Runtime execution and sanitizer execution are still single-file workflows. Project-level runtime and sanitizer orchestration will be added after build-system detection becomes more complete.

For CMake projects, see [CMAKE.md](CMAKE.md).

## Supported Entry Shapes

CppGauntlet supports entries with `arguments`:

```json
{
  "directory": "/path/to/project",
  "file": "src/main.cpp",
  "arguments": ["clang++", "-std=c++20", "-Iinclude", "-c", "src/main.cpp"]
}
```

It also supports entries with `command`:

```json
{
  "directory": "/path/to/project",
  "file": "src/main.cpp",
  "command": "clang++ -std=c++20 -Iinclude -c src/main.cpp"
}
```

The command-string parser handles common shell quoting, but `arguments` is preferred because it avoids shell parsing ambiguity.
