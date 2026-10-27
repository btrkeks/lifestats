import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import type { GoalData } from '@/types';
import type { Day } from '../day';
import { goalFormValues, useGoalForm } from '../useGoalForm';
import { describeDayResult } from './DayActions';
import { GoalFields, hasInlineFields } from './GoalFields';

const cap: GoalData = { type: 'TimedAmounts', data: { target: 400, is_at_least: false } };
const mg = { kind: 'count', unit: 'mg' } as const;

function day(value: Day['value']): Day {
  return {
    date: '2026-10-27', value, goal: cap, has_goal: true, achieved: value !== null,
    percentage_achieved: value === null ? 0 : 100, override: 'none', status: value === null ? 'open' : 'met',
  };
}

describe('TimedAmounts goal components', () => {
  it('retains the total and goal direction in editable form values', () => {
    expect(goalFormValues(cap)).toEqual({ target: 400, is_at_least: false });
    expect(hasInlineFields('TimedAmounts')).toBe(true);
  });

  it('renders amount fields with the mg unit', () => {
    function Form() {
      const form = useGoalForm('TimedAmounts', cap);
      return createElement(GoalFields, { form, statisticType: 'TimedAmounts', measure: mg });
    }
    const html = renderToStaticMarkup(createElement(Form));
    expect(html).toContain('type="number"');
    expect(html).toContain('name="target"');
    expect(html).toContain('mg</span>');
  });

  it('renders a compact amount form with the cap direction', () => {
    function Form() {
      const form = useGoalForm('TimedAmounts', cap);
      return createElement(GoalFields, { form, statisticType: 'TimedAmounts', measure: mg, compact: true });
    }
    const html = renderToStaticMarkup(createElement(Form));
    expect(html).toContain('aria-label="At most"');
    expect(html).toContain('aria-label="Target"');
  });

  it('distinguishes unsent days, confirmed zeros and daily totals in day descriptions', () => {
    expect(describeDayResult(day(null), mg)).toBe('nothing logged (0%)');
    expect(describeDayResult(day(0), mg)).toBe('0 of 400 mg (100%)');
    expect(describeDayResult(day(180), mg)).toBe('180 of 400 mg (100%)');
  });
});
