// Keep application identity consistent across the locked desktop build inputs.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
const root = new URL('../', import.meta.url);
const read = (path) => readFileSync(new URL(path, root), 'utf8');
const json = (path) => JSON.parse(read(path));
const pkg = json('package.json');
const cargo = read('src-tauri/Cargo.toml').match(/^\[package\]\s*\n([\s\S]*?)(?=^\[)/m)?.[1];
const locked = read('src-tauri/Cargo.lock').split('[[package]]').find((block) => block.includes(`name = "${pkg.name}"`));
const version = (block) => block?.match(/^version = "([^"]+)"/m)?.[1];
const versions = {
  'package.json': pkg.version,
  'package-lock.json': json('package-lock.json').version,
  'package-lock.json root': json('package-lock.json').packages[''].version,
  'Cargo.toml': version(cargo),
  'Cargo.lock application': version(locked),
  'tauri.conf.json': json('src-tauri/tauri.conf.json').version,
};
if (!/^\d+\.\d+\.\d+$/.test(pkg.version) || Object.values(versions).some((v) => v !== pkg.version)) {
  console.error('Desktop version mismatch:', versions);
  process.exitCode = 1;
} else {
  console.log(`Desktop version ${pkg.version}: all ${Object.keys(versions).length} build inputs agree (${fileURLToPath(root)}).`);
}
