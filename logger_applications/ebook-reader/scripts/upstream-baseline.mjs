/* global process, console */
// Runs svelte-check or tsc and fails on any diagnostic that upstream 68c40f1c does not already
// produce. Upstream fails both checks; fixing its files would widen the rebase surface.
// usage: node upstream-baseline.mjs svelte-check|tsc [--update]
import { spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const [tool, flag] = process.argv.slice(2);
const commands = {
  'svelte-check': ['svelte-check', '--tsconfig', './tsconfig.json', '--output', 'machine'],
  tsc: ['tsc', '--noEmit', '--pretty', 'false']
};
if (!commands[tool]) {
  console.error('usage: upstream-baseline.mjs svelte-check|tsc [--update]');
  process.exit(2);
}

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const baselineFile = join(root, 'scripts', `upstream-diagnostics.${tool}.txt`);
const shown = relative(process.cwd(), baselineFile);
const run = spawnSync('pnpm', ['exec', ...commands[tool]], { encoding: 'utf8' });
const output = `${run.stdout}${run.stderr}`.replaceAll(process.cwd(), '.').replaceAll(root, '..');

/** Line numbers move with every upstream rebase, so a diagnostic is its file and message. */
const found = new Set(
  tool === 'tsc'
    ? [...output.matchAll(/^(\S.*?)\(\d+,\d+\): (error|warning) (TS\d+: .*)$/gm)].map(
        ([, file, level, message]) => `${level} ${file} ${message}`
      )
    : [...output.matchAll(/^\d+ (ERROR|WARNING) "([^"]+)" \d+:\d+ "(.*)"$/gm)].map(
        ([, level, file, message]) => `${level.toLowerCase()} ${file} ${message}`
      )
);

if (flag === '--update') {
  writeFileSync(baselineFile, [...found].sort().join('\n') + '\n');
  console.log(`wrote ${found.size} diagnostics to ${shown}`);
  process.exit(0);
}

const baseline = new Set(readFileSync(baselineFile, 'utf8').split('\n').filter(Boolean));
const added = [...found].filter((entry) => !baseline.has(entry));
const gone = [...baseline].filter((entry) => !found.has(entry));
added.forEach((entry) => console.error(`new diagnostic: ${entry}`));
gone.forEach((entry) => console.error(`baseline diagnostic no longer reported: ${entry}`));
if (run.status !== 0 && found.size === 0) {
  console.error(output);
  process.exit(run.status ?? 1);
}
if (added.length || gone.length) process.exit(1);
console.log(`${tool}: only the ${baseline.size} upstream diagnostics in ${shown}`);
