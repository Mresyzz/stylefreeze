#!/usr/bin/env node

const { spawnSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const binName = process.platform === 'win32' ? 'stylefreeze.exe' : 'stylefreeze';

const candidatePaths = [
  path.join(__dirname, '..', binName),
  path.join(__dirname, '..', 'target', 'release', binName),
  path.join(__dirname, '..', 'target', 'debug', binName),
];

let targetBin = candidatePaths.find(p => fs.existsSync(p));

const args = process.argv.slice(2);

if (targetBin) {
  const result = spawnSync(targetBin, args, { stdio: 'inherit' });
  process.exit(result.status ?? 0);
} else {
  // If no prebuilt binary found, fallback to cargo run
  const result = spawnSync('cargo', ['run', '--quiet', '--', ...args], {
    cwd: path.join(__dirname, '..'),
    stdio: 'inherit',
  });
  process.exit(result.status ?? 0);
}
