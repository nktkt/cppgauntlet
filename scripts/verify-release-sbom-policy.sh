#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: $0 <sbom.cdx.json>" >&2
  exit 2
fi

sbom="$1"
if [ ! -f "$sbom" ]; then
  echo "release SBOM not found: $sbom" >&2
  exit 1
fi

metadata="$(mktemp)"
trap 'rm -f "$metadata"' EXIT

cargo metadata --locked --format-version 1 > "$metadata"

node - "$metadata" "$sbom" <<'NODE'
const fs = require("fs");

const metadataPath = process.argv[2];
const sbomPath = process.argv[3];
const metadata = JSON.parse(fs.readFileSync(metadataPath, "utf8"));
const bom = JSON.parse(fs.readFileSync(sbomPath, "utf8"));
const failures = [];

function fail(message) {
  failures.push(message);
}

function refFor(pkg) {
  return `pkg:cargo/${pkg.name}@${pkg.version}`;
}

function array(value, path) {
  if (!Array.isArray(value)) {
    fail(`${path} must be an array`);
    return [];
  }
  return value;
}

function requireField(value, field, path) {
  if (value[field] === undefined || value[field] === null || value[field] === "") {
    fail(`${path}.${field} is required`);
  }
}

function compareString(actual, expected, path) {
  if (actual !== expected) {
    fail(`${path} must be ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}

const root = metadata.packages.find((pkg) => pkg.id === metadata.resolve.root);
if (!root) {
  fail("cargo metadata root package is missing");
}

const packagesById = new Map(metadata.packages.map((pkg) => [pkg.id, pkg]));
const expectedPackagesByRef = new Map(metadata.packages.map((pkg) => [refFor(pkg), pkg]));
const rootRef = root ? refFor(root) : "";

compareString(bom.bomFormat, "CycloneDX", "bom.bomFormat");
compareString(bom.specVersion, "1.5", "bom.specVersion");
if (!Number.isInteger(bom.version) || bom.version < 1) {
  fail("bom.version must be a positive integer");
}
if (!/^urn:uuid:[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(bom.serialNumber || "")) {
  fail("bom.serialNumber must be a UUID URN");
}

const metadataComponent = bom.metadata && bom.metadata.component;
if (!metadataComponent) {
  fail("bom.metadata.component is required");
} else if (root) {
  compareString(metadataComponent.type, "application", "bom.metadata.component.type");
  compareString(metadataComponent.name, root.name, "bom.metadata.component.name");
  compareString(metadataComponent.version, root.version, "bom.metadata.component.version");
  compareString(metadataComponent["bom-ref"], rootRef, "bom.metadata.component.bom-ref");
  compareString(metadataComponent.purl, rootRef, "bom.metadata.component.purl");
  requireField(metadataComponent, "licenses", "bom.metadata.component");
}

const tools = array(bom.metadata && bom.metadata.tools, "bom.metadata.tools");
if (!tools.some((tool) => tool.vendor === "CppGauntlet" && tool.name === "scripts/generate-release-sbom.sh")) {
  fail("bom.metadata.tools must include scripts/generate-release-sbom.sh");
}

const components = array(bom.components, "bom.components");
const componentsByRef = new Map();
for (const [index, component] of components.entries()) {
  const path = `bom.components[${index}]`;
  for (const field of ["type", "bom-ref", "name", "version", "purl"]) {
    requireField(component, field, path);
  }
  if (component["bom-ref"] !== component.purl) {
    fail(`${path}.bom-ref must match ${path}.purl`);
  }
  if (componentsByRef.has(component["bom-ref"])) {
    fail(`duplicate component bom-ref: ${component["bom-ref"]}`);
  }
  componentsByRef.set(component["bom-ref"], component);
}

if (componentsByRef.has(rootRef)) {
  fail("root package must be represented by metadata.component, not components[]");
}

const expectedComponentRefs = new Set(
  metadata.packages
    .filter((pkg) => root && pkg.id !== root.id)
    .map(refFor)
);
if (componentsByRef.size !== expectedComponentRefs.size) {
  fail(`components[] count must match locked dependency count: expected ${expectedComponentRefs.size}, got ${componentsByRef.size}`);
}

for (const ref of componentsByRef.keys()) {
  if (!expectedComponentRefs.has(ref)) {
    fail(`unexpected component in SBOM: ${ref}`);
  }
}

for (const [ref, pkg] of expectedPackagesByRef.entries()) {
  if (ref === rootRef) {
    continue;
  }
  const component = componentsByRef.get(ref);
  if (!component) {
    fail(`missing component for locked package: ${ref}`);
    continue;
  }
  compareString(component.type, "library", `${ref}.type`);
  compareString(component.name, pkg.name, `${ref}.name`);
  compareString(component.version, pkg.version, `${ref}.version`);
  compareString(component.purl, ref, `${ref}.purl`);
  if (pkg.license && !array(component.licenses, `${ref}.licenses`).some((license) => license.expression === pkg.license)) {
    fail(`${ref} must carry license expression ${pkg.license}`);
  }
  if (pkg.source) {
    const references = array(component.externalReferences, `${ref}.externalReferences`);
    if (!references.some((entry) => entry.type === "distribution" && entry.url === pkg.source)) {
      fail(`${ref} must carry distribution externalReference ${pkg.source}`);
    }
  }
}

const knownRefs = new Set(expectedPackagesByRef.keys());
const dependencies = array(bom.dependencies, "bom.dependencies");
const dependenciesByRef = new Map();
for (const [index, dependency] of dependencies.entries()) {
  const path = `bom.dependencies[${index}]`;
  requireField(dependency, "ref", path);
  if (!knownRefs.has(dependency.ref)) {
    fail(`${path}.ref is not a known package: ${dependency.ref}`);
  }
  if (dependenciesByRef.has(dependency.ref)) {
    fail(`duplicate dependency ref: ${dependency.ref}`);
  }
  dependenciesByRef.set(dependency.ref, dependency);
  for (const dependsOn of array(dependency.dependsOn, `${path}.dependsOn`)) {
    if (!knownRefs.has(dependsOn)) {
      fail(`${path}.dependsOn contains unknown package: ${dependsOn}`);
    }
  }
}

const expectedDependencies = new Map();
for (const node of metadata.resolve.nodes || []) {
  const pkg = packagesById.get(node.id);
  if (!pkg) {
    fail(`metadata resolve node package missing: ${node.id}`);
    continue;
  }
  expectedDependencies.set(
    refFor(pkg),
    node.deps.map((dep) => refFor(packagesById.get(dep.pkg))).sort()
  );
}

if (dependenciesByRef.size !== expectedDependencies.size) {
  fail(`dependencies[] count must match cargo resolve node count: expected ${expectedDependencies.size}, got ${dependenciesByRef.size}`);
}

for (const [ref, expectedDependsOn] of expectedDependencies.entries()) {
  const dependency = dependenciesByRef.get(ref);
  if (!dependency) {
    fail(`missing dependency entry for ${ref}`);
    continue;
  }
  const actualDependsOn = array(dependency.dependsOn, `${ref}.dependsOn`).sort();
  if (JSON.stringify(actualDependsOn) !== JSON.stringify(expectedDependsOn)) {
    fail(`${ref} dependency list does not match cargo metadata`);
  }
}

if (failures.length > 0) {
  console.error("SBOM policy check failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log(`SBOM policy check passed: ${sbomPath}`);
NODE
