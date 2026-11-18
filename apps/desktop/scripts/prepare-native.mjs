/**
 * Copyright (c) 2025 Bytedance Ltd. and/or its affiliates
 * SPDX-License-Identifier: MIT
 */

import { spawn, spawnSync } from 'node:child_process';
import { chmodSync, copyFileSync, mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const production = process.argv[2] === 'build';
function run(command, args, cwd = root) {
  const result = spawnSync(command, args, { cwd, stdio: 'inherit' });
  if (result.status !== 0) process.exit(result.status || 1);
}
run('cargo', ['build', '--locked', '-p', 'flowgram-desktop-backend', ...(production ? ['--release'] : [])]);
const target = spawnSync('rustc', ['-vV'], { encoding: 'utf8' }).stdout.match(/^host: (.+)$/m)[1];
const suffix = process.platform === 'win32' ? '.exe' : '';
const binaries = resolve(root, 'apps/desktop/src-tauri/binaries');
mkdirSync(binaries, { recursive: true });
for (const binary of ['flowgram-server', 'flowgram-service']) {
  const destination = resolve(binaries, `${binary}-${target}${suffix}`);
  copyFileSync(resolve(root, `target/${production ? 'release' : 'debug'}/${binary}${suffix}`), destination);
  // copyFileSync does not carry the source mode; Tauri's sidecars must stay executable on Unix.
  if (process.platform !== 'win32') chmodSync(destination, 0o755);
}
if (production) run('node', ['../../common/scripts/install-run-rushx.js', 'build'], resolve(root, 'apps/desktop'));
else {
  const child = spawn('node', ['../../common/scripts/install-run-rushx.js', 'dev'], { cwd: resolve(root, 'apps/desktop'), stdio: 'inherit' });
  process.on('SIGTERM', () => child.kill('SIGTERM'));
  process.on('SIGINT', () => child.kill('SIGINT'));
  child.on('exit', (code) => process.exit(code || 0));
}
