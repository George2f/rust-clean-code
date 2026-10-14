import { readFileSync } from 'node:fs';

const cargoToml = readFileSync(new URL('../Cargo.toml', import.meta.url), 'utf8');
const version = cargoToml.match(/^version = "([^"]+)"/m)?.[1];

if (!version) {
  console.error('version: no workspace version found in Cargo.toml');
  process.exit(1);
}

console.log(version);
