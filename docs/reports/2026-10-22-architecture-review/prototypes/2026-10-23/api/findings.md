# API prototype findings

Status is ISSUES. The complete runner exits 0. The toolchain and generated consumer compile. The observed contracts still disagree. This is a throwaway prototype, not production implementation.

The source baseline is `607978998c72b1e14a95518d32fbfeafe2f7fed6`. The parent checkout's HEAD matched that revision. Uncommitted architecture documents were read as directed.

The bounded recommendation is safe-range JSON integers for ordinary amounts and opaque strings for revisions. Keep decimal strings where the full signed i64 range is a product requirement. This experiment does not establish the product's maximum amount or authorize a legacy wire change.

## Reproduce

```sh
bash /home/daniel/.local/state/lifestats-architecture-review/2026-10-23-prototypes/api/run.sh
```

`run.sh` installs from `package-lock.json`, builds only the standalone crate using `rust/Cargo.lock`, emits schemas and validators, compiles `consumer.ts`, and prints observations. `RUSTC_WRAPPER` is empty. The Cargo target and npm installation remain inside this slice. Command failures exit nonzero. Undesirable outcomes remain observations and exit zero.

`results.json` records raw JSON, Rust acceptance and serialized output, JS parsed content, generated runtime acceptance, decoded content and round trips, errors, and comparisons. `console.log` contains short traces. `fixtures/` retains 158 independent raw JSON files. Boundary numbers never pass through `JSON.stringify` before Rust parses them.

## Real generator paths

Path A uses openapi-typescript 7.10.1 and Ajv 8.17.1 standalone generation. Ajv uses JSON Schema 2020-12 and ajv-formats 3.0.1 in full mode. `generate-ajv.cjs` embeds utoipa's components unchanged and selects roots with `$ref`. No schema field is patched. `strict:false` allows OpenAPI annotations. Coercion and property removal are disabled. `int64` is explicitly annotation-only and adds no range check. `generated/validators.cjs` is actual generated executable code.

Path B uses openapi-zod-client 1.18.3, Zod 3.25.76, and default object policy. `generated/zod.ts` is unedited generator output. The [official repository](https://github.com/astahmer/openapi-zod-client) marks this generator deprecated. It is a comparison baseline, not a maintained dependency recommendation.

Commands follow the official [openapi-typescript CLI](https://openapi-ts.dev/cli) and [Ajv standalone generation](https://ajv.js.org/standalone.html) documentation.

The Rust crate pins utoipa 5.5.0, serde 1.0.228, serde_json 1.0.145, chrono 0.4.45, and serde_path_to_error 0.1.20. The utoipa version matches the web API's 5.5 line. The emitted document is OpenAPI 3.1.0. `results.json` reports actual compiler and installed npm versions. Both lockfiles retain exact resolutions.

## Observed comparison

Rust accepts 64 of 158 fixtures and rejects 94. Both generated paths decode all 64 Rust-serialized accepted outputs. Both validate all 94 rejection envelopes emitted by the runner.

The valid amount, time-span, and habit variants retain their tags. Inherited, explicit, and suppressed goals survive decoding. Suppression remains `{"kind":"suppressed"}` rather than becoming `{}`. Required-nullable `note` accepts null and rejects absence in all paths. Optional-nullable `label` accepts absence and null. Rust serializes `label:null` as omission under its declared `skip_serializing_if` policy. Zod retains explicit null.

`consumer.ts` compiles with actual TypeScript. It exhaustively narrows the generated value and goal unions. An active `@ts-expect-error` verifies that the declaration's suppressed variant has no `value`. The generated Zod union narrows tags but allows an unknown `value` through its passthrough index signature. `typecheck-zod-suppression-probe.log` retains the initial unused expectation diagnostic. The final consumer explicitly types that Zod property as `unknown` and compiles.

Counts describe this fixture matrix, not quality scores.

- Contract has 42 fixtures. Rust accepts 13, Ajv 20, and Zod 24. Rust disagrees with Ajv on 9 and with Zod on 11.
- SafeNumber has 16 fixtures. Rust accepts 3, Ajv 7, and Zod 16.
- RawI64Number has 16 fixtures. Rust accepts 9, Ajv 15, and Zod 16.
- DecimalI64 has 20 fixtures. Rust accepts 7, Ajv 10, and Zod 19.
- DecimalAmount has 20 fixtures. Rust accepts 7, Ajv 10, and Zod 10.
- DecimalRevision has 20 fixtures. Rust accepts 5, Ajv 7, and Zod 19.
- OpaqueRevision has 20 fixtures. Rust accepts 18, Ajv 18, and Zod 19.
- ErrorEnvelope has four fixtures. All paths accept the two valid and reject the two invalid examples.

### Numeric precision and parser semantics

The numeric amount variant checks +/-9007199254740991. Rust, Ajv, and the nested generated Zod amount reject outside that range. Valid positive and negative endpoints round-trip exactly.

Raw full-i64 JSON numbers lose digits in JS. Rust round-trips `9007199254740993`. JS round-trips `9007199254740992`. Rust's i64 maximum is `9223372036854775807`, while JS serializes its rounded value as `9223372036854776000`. Both generated paths accept those parsed values. An int64 annotation does not preserve digits or establish bounds.

Rust rejects raw `1.0`, `1e0`, and `-0` for these i64 DTOs. Schema integer means a mathematical integer and accepts the integral JS values after parsing. Raw `9007199254740990.5` also rounds to an integer before validation. A later `Number.isSafeInteger` cannot recover that lost fraction. These are raw parser differences. Ordinary JS serialization of an integral safe value already writes canonical integer JSON.

Canonical decimal strings preserve both i64 endpoints and values beyond JS's safe range. The emitted regex rejects noncanonical examples in Ajv and the nested Zod decimal amount. It does not enforce arithmetic range. Both generated validators accept the above-i64, below-i64, and long all-nines strings that Rust rejects.

The exported Zod scalar roots lose refinements. SafeNumber is only `z.number()`, DecimalI64 and DecimalRevision are only `z.string()`, and OpaqueRevision loses minLength. Constraints appear at some referencing fields. Root and nested outcomes are therefore recorded separately.

`additionalDecimalAuthority` in `observe.cjs` is an explicit handwritten second authority. It checks canonical full-string grammar and BigInt bounds. Combined with either generated path, it agrees with Rust for all 60 decimal and decimal-revision fixtures. It is not generated. Keeping it aligned with the Rust constructor creates drift risk. Raw generated results remain unaltered.

OpaqueRevision deliberately means any nonempty string, with no arithmetic meaning. This prototype does not establish a production token alphabet.

### Unknown properties, suppression, and duplicate keys

The Contract schema has `additionalProperties:false`. Ajv rejects an unknown envelope key. Generated Zod accepts and strips that key. Content comparison sorts object keys, so stripping is distinguished from property reordering.

utoipa omits `additionalProperties:false` from internally tagged enum object variants despite `deny_unknown_fields`. Rust rejects an unknown amount field, while both generated validators accept and retain it. That gap exists in OpenAPI before TS generation.

Serde's unit Suppressed variant accepts an attached value despite `deny_unknown_fields`, then drops it during serialization. Ajv and Zod accept and retain it. Acceptance agrees while decoded content differs. Static union narrowing alone does not prove strict suppression semantics.

Rust rejects duplicate values, notes, and tags. `JSON.parse` keeps the last key before either generated validator runs. Both then accept the resulting value. Parsed-object validation cannot prove duplicate-key agreement. Strict duplicate rejection requires a raw parser policy if that guarantee is needed.

### Dates, timestamps, and structured errors

Rust chrono and Ajv reject February 30. The generated Zod date is plain `z.string()` and accepts it. Chrono accepts `2026-1-2` and normalizes it to `2026-01-02`; Ajv rejects the unpadded input. All paths reject a timestamp missing an offset. Rust and Ajv reject `+25:00`, while the generated Zod datetime check accepts it. This matrix is not exhaustive RFC 3339 verification.

All paths accept a reversed time span. Interval ordering is domain validation outside this prototype.

The scratch error mapper emits invalid_input, out_of_range, or invalid_decimal with field, message, and retryable:false. The enum also includes stale and operation_payload_mismatch examples. Library message text is not a stable client interface. Field paths through tagged-enum buffering can be coarse. No real HTTP rejection mapper or retry classifier is exercised.

## Recommendation and limits

Rust transport DTOs plus utoipa remain a useful contract authority. openapi-typescript plus Ajv standalone is the better observed path here. Its invalid-input behavior follows the emitted schema more consistently than this Zod generator. Neither path proves Rust-parser equivalence or production readiness.

Use bounded JSON numbers where product range permits them, opaque revision tokens, and full-i64 decimal strings only where the range is required. Declare any additional decimal refinement as another authority and cover it with shared fixtures. A schema representation that encodes the arithmetic bound could avoid that refinement, but this prototype does not implement it.

Before implementation, decide lexical integer and duplicate-key policy. Make enum unknown-field and suppression policy explicit in Rust and the emitted schema. Fix representation at the source or disclose any transform as another maintained authority. Decide canonical date parsing. Recheck root validators and format behavior in any selected replacement generator.

Setup also exposed genuine compatibility details. Utoipa tuple-newtype derive rejected minimum, maximum, and minimum-length attributes in both tried positions. SafeNumber and OpaqueRevision therefore use explicit Rust schema builders. These are handwritten protocol declarations, not patches to generated OpenAPI. SafeNumber's schema and parser share the SAFE constant. Naming the local enum Value caused an OpenAPI macro failure; MetricValue compiled.

Model the Domain shaped the tagged value and goal variants. Type System Discipline shaped checked boundary constructors and generated TS consumers. Technical Writing and Unslop shaped this report.

No React Native device, Hermes, Metro bundle, HTTP extractor, route coverage, authentication, SQLite migration, deployment, live API, or real user data was exercised. No production source or service was edited. No performance claim follows. Network access was limited to package registries and official research documentation.

## Artifacts

The exact source and results manifest is `source-results.sha256`. Retain shape.md, findings.md, run.sh, rust/Cargo.toml, rust/Cargo.lock, rust/src/main.rs, package.json, package-lock.json, tsconfig.json, consumer.ts, generate-ajv.cjs, observe.cjs, openapi.json, generated/, fixtures/, rust-results.json, results.json, and logs.

Exclude node_modules/ and rust/target/ from the parent's archive. The runner recreates them. Writes stayed under the assigned API directory, apart from normal package caches.
