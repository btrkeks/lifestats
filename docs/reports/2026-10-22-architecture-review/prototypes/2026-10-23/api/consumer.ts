import type { components } from './generated/openapi';
import { schemas } from './generated/zod';
import type { z } from 'zod';

export function describeValue(value: components['schemas']['MetricValue']): string {
  switch (value.kind) {
    case 'amount': {
      const amount: number = value.value;
      return String(amount + 1);
    }
    case 'time_span': return `${value.start}/${value.end}`;
    case 'habit': return value.completed ? 'done' : 'pending';
    default: {
      const exhaustive: never = value;
      return exhaustive;
    }
  }
}

export function describeContract(contract: components['schemas']['Contract']): string {
  const note: string | null = contract.note;
  const label: string | null | undefined = contract.label;
  switch (contract.goal.kind) {
    case 'inherited': return `inherited ${note} ${label}`;
    case 'explicit': return describeValue(contract.goal.value);
    case 'suppressed': {
      // A suppression cannot carry a goal value in the generated union.
      // @ts-expect-error Suppressed has no value.
      const forbidden = contract.goal.value;
      return `suppressed ${String(forbidden)}`;
    }
    default: {
      const exhaustive: never = contract.goal;
      return exhaustive;
    }
  }
}

export function describeZodGoal(goal: z.infer<typeof schemas.Goal>): string {
  switch (goal.kind) {
    case 'inherited': return 'inherited';
    case 'explicit': return goal.value.kind;
    case 'suppressed': {
      const forbidden: unknown = goal.value;
      return `suppressed ${String(forbidden)}`;
    }
    default: {
      const exhaustive: never = goal;
      return exhaustive;
    }
  }
}

export const suppressed = {
  activity_date: '2026-10-23', observed_at: '2026-10-23T08:00:00+02:00',
  value: { kind: 'habit', completed: true }, goal: { kind: 'suppressed' }, note: null,
} satisfies components['schemas']['Contract'];
