# API prototype data shapes

Decision. Does Rust transport DTO -> utoipa 5.5 OpenAPI -> generated TS types and runtime validation preserve the intended contract?

Payload is a closed envelope with an activity day, offset timestamp, a kind-tagged Value and a kind-tagged Goal. Value variants are amount with a checked safe JSON integer, time_span with two offset timestamps, and habit with a boolean. Goal variants are inherited, explicit with Value, and suppressed without a value. The envelope includes required nullable note and optional nullable label to distinguish presence from null.

Alternative integer roots are SafeNumber, RawI64Number, DecimalI64 and OpaqueRevision. SafeNumber bounds ordinary values to +/-9007199254740991. RawI64Number exposes the existing full-i64 JSON-number risk. DecimalI64 preserves canonical signed full i64 strings and checks arithmetic range in Rust. OpaqueRevision is a nonempty string which the consumer does not perform arithmetic on. DecimalRevision separately explores an opaque-to-consumer revision serialized as a canonical nonnegative i64 decimal string.

Path A uses openapi-typescript declarations and Ajv 2020 standalone generated runtime JS. Path B uses openapi-zod-client emitted Zod schemas without patches. Each consumes exactly the same utoipa document. No production source is imported or changed. Invalid fixtures are observations, not test assertions.

Structured errors contain stable code, field, message and retryable in an error envelope. The scratch runner maps deserialization failures to invalid_input and semantic constructors to out_of_range or invalid_decimal. That mapping is illustrative and is not an HTTP extractor implementation.

Model the Domain shaped the tagged value and goal unions. Type System Discipline shaped checked boundary types and generation from transport schemas. Throughput checkpoint. Generate one fixture matrix once in Rust, then run both real generated validators over the same raw files. No per-case hand decoder.
