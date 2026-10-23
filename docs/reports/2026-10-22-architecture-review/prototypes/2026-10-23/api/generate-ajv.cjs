const fs = require('node:fs');
const Ajv2020 = require('ajv/dist/2020').default;
const addFormats = require('ajv-formats');
const standalone = require('ajv/dist/standalone').default;
const spec = JSON.parse(fs.readFileSync('openapi.json', 'utf8'));
const rustReport = JSON.parse(fs.readFileSync('rust-results.json', 'utf8'));
const roots = [...new Set(rustReport.cases.map(c => c.root))];
const generatorVersion = require('ajv/package.json').version;
const ajv = new Ajv2020({ strict: false, allErrors: true, code: { source: true }, validateFormats: true });
addFormats(ajv, { mode: 'full' });
ajv.addFormat('int64', true);
const validatorIds = {};
for (const root of roots) {
  const id = `https://scratch.invalid/${root}`;
  const schema = { $schema: 'https://json-schema.org/draft/2020-12/schema', $id: id, $ref: `#/components/schemas/${root}`, components: spec.components };
  fs.writeFileSync(`generated/${root}.schema.json`, JSON.stringify(schema, null, 2));
  ajv.addSchema(schema);
  validatorIds[root] = id;
}
fs.writeFileSync('generated/validators.cjs', standalone(ajv, validatorIds));
fs.writeFileSync('generated/ajv-options.json', JSON.stringify({ source_revision: rustReport.source_revision, command: 'node generate-ajv.cjs', generator: 'ajv standalone', version: generatorVersion, dialect: '2020-12', strict: false, allErrors: true, coercion: false, removeAdditional: false, validateFormats: 'ajv-formats full', int64Format: 'annotation only, no range validation', schemaExtraction: 'unmodified components embedded with root $ref; no schema field patches' }, null, 2));
console.log(`Source ${rustReport.source_revision}. Ajv ${generatorVersion} standalone emitted ${roots.length} validators from unmodified components. Command: node generate-ajv.cjs`);
