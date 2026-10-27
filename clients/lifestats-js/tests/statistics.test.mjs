import assert from 'node:assert/strict';
import { test } from 'node:test';
import { LifeStatsClient, statisticTypeSchema, statisticMetadataSchema } from '../dist/index.js';

const intake = {
  id: 42, name: '[OpenNutriTracker] Caffeine (mg)', category: 'OpenNutriTracker', statistic_type: 'TimedAmounts',
};
const habit = { id: 1, name: 'Read', category: 'Habits', statistic_type: 'Habit' };

function respond(t, body) {
  t.mock.method(globalThis, 'fetch', async () => Response.json(body));
}

test('TimedAmounts is accepted by the known-type and metadata schemas', () => {
  assert.equal(statisticTypeSchema.parse('TimedAmounts'), 'TimedAmounts');
  assert.deepEqual(statisticMetadataSchema.parse(intake), intake);
});

test('statistics listing keeps old and TimedAmounts entries and skips unknown types', async (t) => {
  respond(t, [{ statistic_type: 'FutureType' }, habit, intake, { statistic_type: 'AnotherFutureType' }]);
  assert.deepEqual(await new LifeStatsClient().getAllStatistics(), [habit, intake]);
});

test('known-type validation still rejects malformed entries', async (t) => {
  respond(t, [{ ...intake, id: '42' }]);
  await assert.rejects(new LifeStatsClient().getAllStatistics());
});

test('malformed list envelopes still fail validation', async (t) => {
  respond(t, [{ statistic_type: 42 }]);
  await assert.rejects(new LifeStatsClient().getAllStatistics());
});

test('an all-unknown listing succeeds with an empty list', async (t) => {
  respond(t, [{ statistic_type: 'FutureType' }]);
  assert.deepEqual(await new LifeStatsClient().getAllStatistics(), []);
});

test('disabling validation preserves the existing raw-response behavior', async (t) => {
  const unknown = { ...habit, statistic_type: 'FutureType' };
  respond(t, [unknown, intake]);
  assert.deepEqual(await new LifeStatsClient({ validateResponses: false }).getAllStatistics(), [unknown, intake]);
});
