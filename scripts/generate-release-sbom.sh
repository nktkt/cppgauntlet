#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: $0 <output.cdx.json>" >&2
  exit 2
fi

output="$1"
metadata="$(mktemp)"
trap 'rm -f "$metadata"' EXIT

cargo metadata --locked --format-version 1 > "$metadata"
mkdir -p "$(dirname "$output")"

node - "$metadata" "$output" <<'NODE'
const fs = require("fs");
const crypto = require("crypto");

const metadataPath = process.argv[2];
const outputPath = process.argv[3];
const metadata = JSON.parse(fs.readFileSync(metadataPath, "utf8"));
const root = metadata.packages.find((pkg) => pkg.id === metadata.resolve.root);
const packagesById = new Map(metadata.packages.map((pkg) => [pkg.id, pkg]));

function refFor(pkg) {
  return `pkg:cargo/${pkg.name}@${pkg.version}`;
}

function component(pkg, type) {
  const item = {
    type,
    "bom-ref": refFor(pkg),
    name: pkg.name,
    version: pkg.version,
    purl: refFor(pkg)
  };

  if (pkg.license) {
    item.licenses = [{ expression: pkg.license }];
  }
  if (pkg.source) {
    item.externalReferences = [{ type: "distribution", url: pkg.source }];
  }

  return item;
}

const rootDependencies = new Map();
for (const node of metadata.resolve.nodes || []) {
  rootDependencies.set(node.id, node.deps.map((dep) => dep.pkg));
}

const bom = {
  bomFormat: "CycloneDX",
  specVersion: "1.5",
  serialNumber: `urn:uuid:${crypto.randomUUID()}`,
  version: 1,
  metadata: {
    timestamp: new Date().toISOString(),
    tools: [
      {
        vendor: "CppGauntlet",
        name: "scripts/generate-release-sbom.sh"
      }
    ],
    component: component(root, "application")
  },
  components: metadata.packages
    .filter((pkg) => pkg.id !== root.id)
    .sort((left, right) => left.name.localeCompare(right.name) || left.version.localeCompare(right.version))
    .map((pkg) => component(pkg, "library")),
  dependencies: Array.from(rootDependencies.entries()).map(([ref, dependsOn]) => ({
    ref: refFor(packagesById.get(ref)),
    dependsOn: dependsOn.map((id) => refFor(packagesById.get(id))).sort()
  }))
};

fs.writeFileSync(outputPath, `${JSON.stringify(bom, null, 2)}\n`);
NODE
