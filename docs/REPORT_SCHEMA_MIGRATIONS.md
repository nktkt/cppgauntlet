# Report Schema Migrations

CppGauntlet report consumers should use the `schema_version` field before assuming that optional fields are present.

The current report schema version is `3`.

## Compatibility Policy

- Unknown fields are non-breaking and should be ignored by consumers.
- New optional fields may be added to the current schema when older consumers can continue reading the report.
- Required field removals, field renames, type changes, and semantic changes require a schema version bump.
- `baseline update` always writes the normalized baseline with the current schema version.
- Compatibility fixtures for older report and baseline files live under `tests/fixtures/reports/`.
- Every schema version bump must update `REPORT_SCHEMA_VERSION`, this migration guide, the report schema reference, and compatibility tests.

## Schema Version 1

Schema version 1 reports predate structured diagnostic collections.

Important shape:

- stage output keeps raw `stdout` and `stderr`
- summary fields include warning and error counts
- summary and stage `diagnostics` fields may be absent

Migration behavior:

- `baseline update` accepts schema version 1 reports
- structured diagnostics are rebuilt from stage stdout/stderr when possible
- the normalized baseline is written as schema version 3
- parsed locations and fingerprints are generated for rebuilt diagnostics when possible

Consumer guidance:

- do not require `summary.diagnostics`
- do not require `stages[].diagnostics`
- keep raw stage output available for re-parsing

Fixture:

- `tests/fixtures/reports/schema-v1-report.json`

## Schema Version 2

Schema version 2 reports introduced structured diagnostics, but diagnostic metadata was still incomplete.

Important shape:

- `summary.diagnostics` is present
- `stages[].diagnostics[]` is present
- diagnostic `location` may be absent
- diagnostic `fingerprint` may be absent

Migration behavior:

- schema version 2 baselines are accepted by `check --baseline`
- baseline comparison backfills diagnostic metadata from raw diagnostic text
- fingerprints are computed before new versus existing diagnostic comparison
- new reports and normalized baselines are written as schema version 3

Consumer guidance:

- prefer `diagnostics[].fingerprint` when it exists
- fall back to stable handling of `diagnostics[].raw` for older reports
- treat `diagnostics[].location` as optional

Fixture:

- `tests/fixtures/reports/schema-v2-baseline.json`

## Schema Version 3

Schema version 3 is the current report contract.

Important shape:

- diagnostics include stable `fingerprint` values when CppGauntlet can derive them
- diagnostics include parsed `location` values when CppGauntlet can parse source location prefixes
- `summary.coverage.changed_lines` is present when changed-line coverage data is available
- SARIF output can use diagnostic fingerprints as partial fingerprints

Consumer guidance:

- use `schema_version` for compatibility branching
- treat `diagnostics[].location` as optional because not every tool output has a source location
- use `diagnostics[].fingerprint` for baseline and SARIF-style identity
- treat `summary.coverage.changed_lines` as optional
- avoid depending on stage order unless the command being inspected requires it

## Future Migration Checklist

Before changing the report contract:

1. Decide whether the change is additive or requires a schema version bump.
2. Update `REPORT_SCHEMA_VERSION` in the Rust report model when a bump is required.
3. Add or update a fixture under `tests/fixtures/reports/`.
4. Add compatibility tests that prove older reports or baselines still deserialize and normalize correctly.
5. Update [REPORT_SCHEMA.md](REPORT_SCHEMA.md) with the current field contract.
6. Update this file with migration behavior and consumer guidance.
7. Update SARIF, baseline, coverage, or artifact docs when those outputs are affected.
8. Include the compatibility impact in the pull request description.

Breaking changes should keep at least one migration path through `baseline update` whenever the old report contains enough raw information to recover the new structure.

## Release Candidate Gate

Before a release candidate is built, the release workflow runs:

```bash
bash scripts/verify-report-schema-compat.sh
```

The script runs the schema compatibility subset of the Rust integration tests:

```bash
cargo test --locked --test cli schema
```

This gate keeps older report and baseline fixtures deserializable before tagged release artifacts are built.
