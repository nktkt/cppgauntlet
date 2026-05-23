#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <archive.tar.gz> <archive.tar.gz.sha256>" >&2
  exit 2
fi

archive="$1"
checksum="$2"

if [ ! -f "$archive" ]; then
  echo "release archive not found: $archive" >&2
  exit 1
fi

if [ ! -f "$checksum" ]; then
  echo "release checksum not found: $checksum" >&2
  exit 1
fi

archive_basename="$(basename "$archive")"
package_name="${archive_basename%.tar.gz}"
if [ "$package_name" = "$archive_basename" ]; then
  echo "release archive must end with .tar.gz: $archive" >&2
  exit 1
fi

checksum_dir="$(dirname "$checksum")"
checksum_file="$(basename "$checksum")"
if command -v shasum >/dev/null 2>&1; then
  (cd "$checksum_dir" && shasum -a 256 -c "$checksum_file" >/dev/null)
else
  (cd "$checksum_dir" && sha256sum -c "$checksum_file" >/dev/null)
fi

manifest="$(mktemp)"
tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/cppgauntlet-release-smoke.XXXXXX")"
trap 'rm -f "$manifest"; rm -rf "$tmpdir"' EXIT

tar -tzf "$archive" | sort > "$manifest"

for required in \
  "$package_name/" \
  "$package_name/cppgauntlet" \
  "$package_name/README.md" \
  "$package_name/LICENSE" \
  "$package_name/INSTALLATION.md" \
  "$package_name/RELEASE.md"
do
  if ! grep -Fx "$required" "$manifest" >/dev/null; then
    echo "release archive is missing $required" >&2
    exit 1
  fi
done

tar -xzf "$archive" -C "$tmpdir"
binary="$tmpdir/$package_name/cppgauntlet"

if [ ! -x "$binary" ]; then
  echo "release binary is not executable: $binary" >&2
  exit 1
fi

"$binary" --version >/dev/null
help_output="$("$binary" --help)"
if ! printf '%s\n' "$help_output" | grep -F "Put C++ code through" >/dev/null; then
  echo "release binary help output does not look like CppGauntlet" >&2
  exit 1
fi

echo "Release archive smoke test passed: $archive_basename"
