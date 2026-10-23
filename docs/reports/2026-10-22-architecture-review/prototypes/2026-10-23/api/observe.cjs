const fs = require('node:fs');
const cp = require('node:child_process');
const validators = require('./generated/validators.cjs');
const { schemas } = require('./generated/zod.ts');
const typecheckOutput = cp.execFileSync('node_modules/.bin/tsc', ['--noEmit'], { encoding: 'utf8' });
fs.writeFileSync('typecheck.log', typecheckOutput);
const installed = JSON.parse(cp.execFileSync('npm', ['ls', '--json', '--depth=0'], { encoding: 'utf8' }));
const rust = JSON.parse(fs.readFileSync('rust-results.json', 'utf8'));

function additionalDecimalAuthority(value, nonnegative) {
  if (typeof value !== 'string') return { accepted: false, reason: 'expected string' };
  const match = /^(0|[1-9][0-9]*|-[1-9][0-9]*)$/.exec(value);
  if (!match || match[0] !== value) return { accepted: false, reason: 'noncanonical decimal' };
  const integer = BigInt(value);
  const min = nonnegative ? 0n : -9223372036854775808n;
  if (integer < min || integer > 9223372036854775807n) return { accepted: false, reason: 'i64 range' };
  return { accepted: true, decimal: integer.toString() };
}

function stableContent(value) {
  if (Array.isArray(value)) return value.map(stableContent);
  if (value !== null && typeof value === 'object') return Object.fromEntries(Object.keys(value).sort().map(key => [key, stableContent(value[key])]));
  return value;
}
function contentEqual(left, right) { return JSON.stringify(stableContent(left)) === JSON.stringify(stableContent(right)); }
const results = [];
for (const fixture of rust.cases) {
  const raw = fs.readFileSync(`fixtures/${fixture.id}.json`, 'utf8');
  let parsed;
  try { parsed = JSON.parse(raw); } catch (error) { throw new Error(`Runner fixture JSON failure ${fixture.id}: ${error.message}`); }
  const ajvAccepted = validators[fixture.root](parsed);
  const ajvErrors = ajvAccepted ? [] : structuredClone(validators[fixture.root].errors);
  const zod = schemas[fixture.root].safeParse(parsed);
  let refinement = null;
  if (['DecimalI64', 'DecimalAmount', 'DecimalRevision'].includes(fixture.root)) {
    refinement = additionalDecimalAuthority(fixture.root === 'DecimalAmount' ? parsed.value : parsed, fixture.root === 'DecimalRevision');
  }
  const rustRoundTrip = fixture.rust.round_trip_raw;
  let serialized = null;
  let errorEnvelope = null;
  if (fixture.rust.accepted) {
    const rustOutput = JSON.parse(rustRoundTrip);
    const outputZod = schemas[fixture.root].safeParse(rustOutput);
    serialized = { raw: rustRoundTrip, js_decoded: rustOutput, ajv_accepted: validators[fixture.root](rustOutput), zod_accepted: outputZod.success, zod_decoded: outputZod.success ? outputZod.data : null };
  } else {
    const envelope = { error: fixture.rust.error };
    errorEnvelope = { envelope, ajv_accepted: validators.ErrorEnvelope(envelope), zod_accepted: schemas.ErrorEnvelope.safeParse(envelope).success };
  }
  results.push({
    ...fixture,
    js_parse: { decoded: parsed, round_trip_raw: JSON.stringify(parsed), raw_is_same_as_js_round_trip: raw === JSON.stringify(parsed) },
    ajv: { accepted: ajvAccepted, decoded: ajvAccepted ? parsed : null, round_trip_raw: ajvAccepted ? JSON.stringify(parsed) : null, errors: ajvErrors },
    zod: { accepted: zod.success, decoded: zod.success ? zod.data : null, round_trip_raw: zod.success ? JSON.stringify(zod.data) : null, changed_js_content: zod.success ? !contentEqual(zod.data, parsed) : null, errors: zod.success ? [] : zod.error.issues },
    rust_serialized_output: serialized,
    rust_rejection_error_envelope: errorEnvelope,
    additional_decimal_authority: refinement,
    additional_decimal_combined: refinement ? { ajv_accepted: ajvAccepted && refinement.accepted, zod_accepted: zod.success && refinement.accepted, agrees_with_rust: fixture.rust.accepted === (ajvAccepted && refinement.accepted) && fixture.rust.accepted === (zod.success && refinement.accepted) } : null,
    agreement: { rust_ajv: fixture.rust.accepted === ajvAccepted, rust_zod: fixture.rust.accepted === zod.success },
    rust_vs_js_numeric_content: fixture.root === 'RawI64Number' && fixture.rust.accepted ? { rust: rustRoundTrip, js: JSON.stringify(parsed), identical: rustRoundTrip === JSON.stringify(parsed) } : null,
  });
}
const summary = {};
for (const root of [...new Set(results.map(r => r.root))]) {
  const rows = results.filter(r => r.root === root);
  summary[root] = { fixtures: rows.length, rust_accepted: rows.filter(r => r.rust.accepted).length, ajv_accepted: rows.filter(r => r.ajv.accepted).length, zod_accepted: rows.filter(r => r.zod.accepted).length, rust_ajv_disagreements: rows.filter(r => !r.agreement.rust_ajv).map(r => r.id), rust_zod_disagreements: rows.filter(r => !r.agreement.rust_zod).map(r => r.id), zod_transformed: rows.filter(r => r.zod.changed_js_content).map(r => r.id) };
}
const packageInfo = JSON.parse(fs.readFileSync('package.json','utf8'));
const versions = { node: process.version, npm: cp.execFileSync('npm',['--version'],{encoding:'utf8'}).trim(), rustc: cp.execFileSync('rustc',['--version'],{encoding:'utf8'}).trim(), cargo: cp.execFileSync('cargo',['--version'],{encoding:'utf8'}).trim(), npm_packages: Object.fromEntries(Object.entries(installed.dependencies).map(([name, info]) => [name, info.version])), npm_requested_pins: packageInfo.devDependencies, rust_direct: { serde: '1.0.228', serde_json:'1.0.145',utoipa:'5.5.0',chrono:'0.4.45',serde_path_to_error:'0.1.20' } };
const output = {
  status:'ISSUES', source_revision:rust.source_revision, versions,
  commands:['bash run.sh','env RUSTC_WRAPPER= CARGO_TARGET_DIR="$PWD/rust/target" cargo run --locked --manifest-path rust/Cargo.toml','node_modules/.bin/openapi-typescript openapi.json -o generated/openapi.d.ts','node generate-ajv.cjs','node_modules/.bin/openapi-zod-client openapi.json -o generated/zod.ts --export-schemas --export-types','node_modules/.bin/tsc --noEmit','node_modules/.bin/tsx observe.cjs'],
  verification: {
    typecheck: 'PASS, tsc --noEmit, generated declarations and generated Zod consumers compiled',
    typecheck_command: 'node_modules/.bin/tsc --noEmit',
    rust_serialized_outputs: { count: results.filter(r=>r.rust.accepted).length, ajv_rejected: results.filter(r=>r.rust_serialized_output && !r.rust_serialized_output.ajv_accepted).map(r=>r.id), zod_rejected: results.filter(r=>r.rust_serialized_output && !r.rust_serialized_output.zod_accepted).map(r=>r.id) },
    structured_rejections: { count: results.filter(r=>!r.rust.accepted).length, rejected_by_generated_envelope: results.filter(r=>r.rust_rejection_error_envelope && (!r.rust_rejection_error_envelope.ajv_accepted || !r.rust_rejection_error_envelope.zod_accepted)).map(r=>r.id) },
    explicit_decimal_refinement: { count: results.filter(r=>r.additional_decimal_authority).length, disagreements: results.filter(r=>r.additional_decimal_combined && !r.additional_decimal_combined.agrees_with_rust).map(r=>r.id) }
  },
  additional_authority:'additionalDecimalAuthority is explicitly handwritten after generated schema validation. It adds canonical full-string grammar and BigInt i64 bounds. It is not generated and must be kept in agreement with Rust constructors.',
  summary, cases:results,
};
fs.writeFileSync('results.json', JSON.stringify(output,null,2)+'\n');
console.log(`Versions ${JSON.stringify(versions)}`);
console.log(`Command: node_modules/.bin/tsx observe.cjs`);
console.log(`API prototype ISSUES. Source ${rust.source_revision}. ${results.length} raw Rust fixtures decoded by generated validators.`);
for (const [root,s] of Object.entries(summary)) console.log(`${root}: Rust ${s.rust_accepted}/${s.fixtures}, Ajv ${s.ajv_accepted}/${s.fixtures}, Zod ${s.zod_accepted}/${s.fixtures}; disagreements Rust/Ajv ${s.rust_ajv_disagreements.length}, Rust/Zod ${s.rust_zod_disagreements.length}.`);
for (const id of ['habit_suppressed','unknown_top','unknown_value','suppression_with_value','duplicate_value','date_impossible','date_not_padded','timestamp_missing_offset','safe_float_lexical','safe_float_near_safe','raw_i64_positive_rounded','raw_i64_i64_max','decimal_above_i64','decimal_trailing_newline']) {
  const r = results.find(r=>r.id===id);
  console.log(JSON.stringify({id, rust:r.rust, ajv:{accepted:r.ajv.accepted,round_trip_raw:r.ajv.round_trip_raw},zod:{accepted:r.zod.accepted,round_trip_raw:r.zod.round_trip_raw},refinement:r.additional_decimal_authority}));
}
