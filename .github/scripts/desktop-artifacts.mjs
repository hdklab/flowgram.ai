/**
 * Copyright (c) 2025 Bytedance Ltd. and/or its affiliates
 * SPDX-License-Identifier: MIT
 */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { appendFileSync, copyFileSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const target = process.env.DESKTOP_TARGET;
const targets = {
  'aarch64-apple-darwin': { extension: '.dmg', directory: 'dmg' },
  'x86_64-apple-darwin': { extension: '.dmg', directory: 'dmg' },
  'x86_64-pc-windows-msvc': { extension: '.exe', directory: 'nsis', suffix: '-setup' },
  'x86_64-unknown-linux-gnu': {
    extension: '.AppImage',
    directory: 'appimage',
    extra: [
      { extension: '.deb', directory: 'deb' },
      { extension: '.rpm', directory: 'rpm' },
    ],
  },
};
assert.ok(Object.hasOwn(targets, target), 'Unsupported desktop target');
assert.ok(process.env.DESKTOP_ARTIFACT_DIR, 'Set DESKTOP_ARTIFACT_DIR');
const { version } = JSON.parse(readFileSync('apps/desktop/src-tauri/tauri.conf.json', 'utf8'));
const { extension, directory: bundleDirectory, suffix = '', extra = [] } = targets[target];
const directory = join('target/release/bundle', bundleDirectory);
const installers = readdirSync(directory).filter((name) => name.endsWith(extension));
assert.equal(installers.length, 1, 'Expected exactly one installer for the native target');
const output = resolve(process.env.DESKTOP_ARTIFACT_DIR);
mkdirSync(output, { recursive: true });
const name = `FlowGram_${version}_${target}${suffix}${extension}`;
const source = join(directory, installers[0]);
const content = readFileSync(source);
assert.ok(content.length > 0, 'Installer is empty');
copyFileSync(source, join(output, name));
const sha256 = createHash('sha256').update(content).digest('hex');
const files = [{ name, sha256 }];
for (const bundled of extra) {
  const extraDirectory = join('target/release/bundle', bundled.directory);
  const found = readdirSync(extraDirectory).filter((file) => file.endsWith(bundled.extension));
  assert.equal(found.length, 1, `Expected exactly one ${bundled.extension} installer`);
  const extraContent = readFileSync(join(extraDirectory, found[0]));
  assert.ok(extraContent.length > 0, 'Installer is empty');
  copyFileSync(join(extraDirectory, found[0]), join(output, found[0]));
  files.push({
    name: found[0],
    sha256: createHash('sha256').update(extraContent).digest('hex'),
  });
}
writeFileSync(join(output, 'SHA256SUMS.txt'),
  `${files.map((file) => `${file.sha256}  ${file.name}`).join('\n')}\n`);
writeFileSync(join(output, 'build-info.json'), `${JSON.stringify({
  version,
  target,
  commit: process.env.GITHUB_SHA || null,
  run: process.env.GITHUB_RUN_ID || null,
  signed: false,
  installer: name,
  bytes: content.length,
  sha256,
}, null, 2)}\n`);
if (process.env.GITHUB_STEP_SUMMARY) {
  appendFileSync(process.env.GITHUB_STEP_SUMMARY,
    `### FlowGram ${version} · ${target}\n\nInstaller: \`${name}\`\n\nSHA-256: \`${sha256}\`\n\nUnsigned build; retained for 14 days.\n`);
}
console.log(`Collected ${files.map((file) => file.name).join(', ')}`);
