// Builds pf-wasm for the browser and generates JS bindings into src/lib/wasm/.
// Requires: Rust (wasm32-unknown-unknown target) and wasm-bindgen-cli matching Cargo.lock.
import { spawnSync } from 'node:child_process';
import { homedir } from 'node:os';
import { delimiter, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(fileURLToPath(new URL('../../..', import.meta.url)));
const outDir = resolve(fileURLToPath(new URL('../src/lib/wasm', import.meta.url)));
const env = { ...process.env, PATH: [join(homedir(), '.cargo', 'bin'), process.env.PATH].join(delimiter) };

function run(cmd, args) {
  const r = spawnSync(cmd, args, { cwd: root, env, stdio: 'inherit' });
  if (r.error) throw r.error;
  if (r.status !== 0) process.exit(r.status ?? 1);
}

run('cargo', ['build', '-p', 'pf-wasm', '--target', 'wasm32-unknown-unknown', '--release']);
run('wasm-bindgen', [
  '--target', 'web',
  // core.ts passes the .wasm URL explicitly; without this Vite would bundle a second copy.
  '--omit-default-module-path',
  '--out-dir', outDir,
  '--out-name', 'pf_wasm',
  join(root, 'target', 'wasm32-unknown-unknown', 'release', 'pf_wasm.wasm'),
]);
console.log(`wasm bindings written to ${outDir}`);
