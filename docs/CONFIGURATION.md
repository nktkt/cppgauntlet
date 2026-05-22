# Configuration

CppGauntlet can read project defaults from `cppgauntlet.yaml`.

Create a starter file:

```bash
cppgauntlet init
```

Overwrite an existing file:

```bash
cppgauntlet init --force
```

Use a non-default path:

```bash
cppgauntlet check main.cpp --config path/to/cppgauntlet.yaml
```

## Precedence

Values are resolved in this order:

1. CLI arguments
2. `cppgauntlet.yaml`
3. built-in defaults

This lets teams commit shared project defaults while individual CI jobs or local runs override specific fields.

## Current Format

```yaml
standard: c++20
compiler: clang++
artifact_dir: .cppgauntlet
timeout_seconds: 30

sanitizers:
  enabled:
    - address
    - undefined

report:
  path: .cppgauntlet/cppgauntlet-report.json
```

## Fields

- `standard`: `c++17`, `c++20`, or `c++23`
- `compiler`: compiler executable, currently expected to be Clang-compatible
- `artifact_dir`: generated build artifacts and default report location
- `timeout_seconds`: per-command timeout
- `sanitizers.enabled`: `address`, `undefined`, `asan`, `ubsan`, or an empty list
- `report.path`: explicit JSON report path

If `report.path` is omitted, CppGauntlet writes to `<artifact_dir>/cppgauntlet-report.json`.
