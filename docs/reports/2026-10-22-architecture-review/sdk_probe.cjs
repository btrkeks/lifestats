// Diagnostic only: actual SDK schemas, transpiled in memory; no network or builds.
const fs = require('node:fs');
const path = require('node:path');
const { createRequire } = require('node:module');
const root = path.resolve(process.argv[2] || path.join(__dirname, '../../..'));
const fromUI = createRequire(path.join(root, 'ui/web-ui/package.json'));
const ts = fromUI('typescript');
const zod = fromUI('zod/v3');
const filename = path.join(root, 'clients/lifestats-js/src/schemas.ts');
const js = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText;
const loaded = { exports: {} };
new Function('require', 'module', 'exports', js)((name) => {
  if (name === 'zod') return zod;
  throw new Error('Unexpected dependency: ' + name);
}, loaded, loaded.exports);
const schema = loaded.exports;
const metadata = schema.statisticMetadataSchema.parse({
  id: 1, name: 'Synthetic', category: 'Test', statistic_type: 'Habit', default_goal: { target: true },
});
const result = {
  valid_updateendvalue_accepted: schema.updatePolicySchema.safeParse('updateendvalue').success,
  unsupported_max_accepted: schema.updatePolicySchema.safeParse('max').success,
  metadata_preserves_default_goal: Object.hasOwn(metadata, 'default_goal'),
  suppressed_daily_goal_result: schema.goalResponseSchema.parse({ kind: 'suppressed' }),
  null_default_goal_accepted: schema.goalResponseSchema.safeParse(null).success,
  scope: 'Actual SDK schema source; installed web UI TypeScript and Zod v3 compatibility export. No HTTP, full SDK build, or RN runtime validation.',
};
if (process.argv[3]) fs.writeFileSync(process.argv[3], JSON.stringify(result, null, 2) + '\n');
console.log(JSON.stringify(result));
