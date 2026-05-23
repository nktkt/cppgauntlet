#!/usr/bin/env bash
set -euo pipefail

extra_args=()
if [ -n "${CPPGAUNTLET_CARGO_PUBLISH_EXTRA_ARGS:-}" ]; then
  read -r -a extra_args <<< "$CPPGAUNTLET_CARGO_PUBLISH_EXTRA_ARGS"
fi

cargo publish --dry-run --locked "${extra_args[@]}"
