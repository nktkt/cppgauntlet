#!/usr/bin/env bash
set -euo pipefail

cargo test --locked --test cli schema
