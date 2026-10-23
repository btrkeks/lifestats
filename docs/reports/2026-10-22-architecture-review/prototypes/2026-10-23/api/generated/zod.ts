import { makeApi, Zodios, type ZodiosOptions } from "@zodios/core";
import { z } from "zod";

type Contract = {
  activity_date: string;
  goal: Goal;
  label?: (string | null) | undefined;
  note: string | null;
  observed_at: string;
  value: MetricValue;
};
type Goal =
  | {
      kind: "inherited";
    }
  | {
      kind: "explicit";
      value: MetricValue;
    }
  | {
      kind: "suppressed";
    };
type MetricValue =
  | {
      kind: "amount";
      value: SafeNumber;
    }
  | {
      end: string;
      kind: "time_span";
      start: string;
    }
  | {
      completed: boolean;
      kind: "habit";
    };
type SafeNumber = number;
type DecimalAmount = {
  kind: "amount";
  value: DecimalI64;
};
type DecimalI64 = string;
type ErrorDetail = {
  code: ErrorCode;
  field: string | null;
  message: string;
  retryable: boolean;
};
type ErrorCode =
  | "invalid_input"
  | "out_of_range"
  | "invalid_decimal"
  | "stale"
  | "operation_payload_mismatch";
type ErrorEnvelope = {
  error: ErrorDetail;
};

const SafeNumber = z.number();
const MetricValue = z.union([
  z
    .object({
      kind: z.literal("amount"),
      value: SafeNumber.int().gte(-9007199254740991).lte(9007199254740991),
    })
    .passthrough(),
  z
    .object({
      end: z.string().datetime({ offset: true }),
      kind: z.literal("time_span"),
      start: z.string().datetime({ offset: true }),
    })
    .passthrough(),
  z.object({ completed: z.boolean(), kind: z.literal("habit") }).passthrough(),
]);
const Goal = z.union([
  z.object({ kind: z.literal("inherited") }).passthrough(),
  z.object({ kind: z.literal("explicit"), value: MetricValue }).passthrough(),
  z.object({ kind: z.literal("suppressed") }).passthrough(),
]);
const Contract: z.ZodType<Contract> = z.object({
  activity_date: z.string(),
  goal: Goal,
  label: z.union([z.string(), z.null()]).optional(),
  note: z.union([z.string(), z.null()]),
  observed_at: z.string().datetime({ offset: true }),
  value: MetricValue,
});
const ErrorCode = z.enum([
  "invalid_input",
  "out_of_range",
  "invalid_decimal",
  "stale",
  "operation_payload_mismatch",
]);
const ErrorDetail: z.ZodType<ErrorDetail> = z.object({
  code: ErrorCode,
  field: z.union([z.string(), z.null()]),
  message: z.string(),
  retryable: z.boolean(),
});
const ErrorEnvelope: z.ZodType<ErrorEnvelope> = z.object({
  error: ErrorDetail,
});
const DecimalI64 = z.string();
const DecimalAmount: z.ZodType<DecimalAmount> = z
  .object({
    kind: z.literal("amount"),
    value: DecimalI64.regex(/^(0|[1-9][0-9]*|-[1-9][0-9]*)$/),
  })
  .passthrough();
const DecimalRevision = z.string();
const OpaqueRevision = z.string();
const RawI64Number = z.number();

export const schemas = {
  SafeNumber,
  MetricValue,
  Goal,
  Contract,
  ErrorCode,
  ErrorDetail,
  ErrorEnvelope,
  DecimalI64,
  DecimalAmount,
  DecimalRevision,
  OpaqueRevision,
  RawI64Number,
};

const endpoints = makeApi([
  {
    method: "post",
    path: "/prototype",
    alias: "prototype",
    requestFormat: "json",
    parameters: [
      {
        name: "body",
        type: "Body",
        schema: Contract,
      },
    ],
    response: Contract,
    errors: [
      {
        status: 400,
        schema: ErrorEnvelope,
      },
    ],
  },
]);

export const api = new Zodios(endpoints);

export function createApiClient(baseUrl: string, options?: ZodiosOptions) {
  return new Zodios(baseUrl, endpoints, options);
}
