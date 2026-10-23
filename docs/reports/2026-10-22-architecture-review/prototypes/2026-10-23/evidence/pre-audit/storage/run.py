#!/usr/bin/env python3
"""Throwaway SQLite storage comparison. Run with python3 run.py --variant all."""

import argparse
import copy
import json
import platform
import sqlite3
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = Path('/home/daniel/.local/share/src/lifestats')
SOURCE_REVISION = '607978998c72b1e14a95518d32fbfeafe2f7fed6'
VARIANTS = ('aggregate_ledger', 'contributor_rows', 'command_log')
D1, D2 = '2026-10-20', '2026-10-21'
MIN, MAX = -(2 ** 63), 2 ** 63 - 1
SCHEMAS = {
    'aggregate_ledger': '''
CREATE TABLE scopes(metric TEXT, day TEXT, ownership TEXT NOT NULL,
  PRIMARY KEY(metric, day));
CREATE TABLE aggregates(metric TEXT, day TEXT, value INTEGER NOT NULL,
  PRIMARY KEY(metric, day));
CREATE TABLE source_ledger(metric TEXT, day TEXT, identity TEXT,
  value INTEGER NOT NULL, revision INTEGER NOT NULL, cleared INTEGER NOT NULL,
  PRIMARY KEY(metric, day, identity));
''',
    'contributor_rows': '''
CREATE TABLE scopes(metric TEXT, day TEXT, ownership TEXT NOT NULL,
  PRIMARY KEY(metric, day));
CREATE TABLE contributions(metric TEXT, day TEXT,
  kind TEXT NOT NULL CHECK(kind IN ('manual','source','legacy_adjustment')),
  identity TEXT NOT NULL, amount_decimal TEXT NOT NULL,
  revision INTEGER NOT NULL, cleared INTEGER NOT NULL,
  PRIMARY KEY(metric, day, kind, identity));
''',
    'command_log': '''
CREATE TABLE commands(seq INTEGER PRIMARY KEY, kind TEXT NOT NULL, payload_json TEXT NOT NULL);
CREATE TRIGGER commands_no_update BEFORE UPDATE ON commands
  BEGIN SELECT RAISE(ABORT, 'append only'); END;
CREATE TRIGGER commands_no_delete BEFORE DELETE ON commands
  BEGIN SELECT RAISE(ABORT, 'append only'); END;
''',
}


def bounded(value, bits=64):
    if not -(2 ** (bits - 1)) <= value < 2 ** (bits - 1):
        raise ValueError(f'i{bits}_range')
    return value


def contribution(value, revision=0, cleared=False):
    return {'amount': int(value), 'revision': revision, 'cleared': bool(cleared)}


def total(scope):
    return bounded(bounded(sum(row['amount'] for row in scope['rows'].values()), 128))


def decide(state, cmd, variant):
    """One semantic reducer keeps the product meanings equal across persistence choices."""
    state = copy.deepcopy(state)
    key = (cmd['metric'], cmd['day'])
    scope = state.setdefault(key, {'ownership': 'open', 'rows': {}})
    rows, kind = scope['rows'], cmd['kind']
    if kind == 'legacy_seed':
        scope['ownership'] = cmd['ownership']
        scope['rows'] = rows = {}
        for source in cmd.get('known_sources', []):
            rows[('source', source['identity'])] = contribution(
                bounded(int(source['value'])), source['revision'])
        residual = bounded(int(cmd['aggregate'])) - sum(r['amount'] for r in rows.values())
        rows[('legacy_adjustment', 'unknown')] = contribution(bounded(residual, 128))
    elif kind == 'source_snapshot':
        identity = ('source', cmd['identity'])
        old = rows.get(identity)
        value = bounded(int(cmd['value']))
        cleared = cmd.get('cleared', False)
        if cleared and value != 0:
            return state, 'invalid_clear'
        if scope['ownership'] == 'mixed' and old is None:
            return state, 'needs_reconciliation'
        if old:
            if cmd['revision'] < old['revision']:
                return state, 'stale'
            if cmd['revision'] == old['revision']:
                same = (value, cleared) == (old['amount'], old['cleared'])
                return state, 'unchanged' if same else 'revision_conflict'
        rows[identity] = contribution(value, cmd['revision'], cleared)
    elif kind == 'manual_add':
        value = bounded(int(cmd['value']))
        if variant == 'aggregate_ledger':
            identity = ('legacy_adjustment', 'unknown')
            rows[identity] = contribution(bounded(rows.get(identity, contribution(0))['amount'] + value, 128))
        else:
            identity = ('manual', 'manual')
            rows[identity] = contribution(bounded(rows.get(identity, contribution(0))['amount'] + value))
    elif kind == 'manual_set':
        if variant == 'aggregate_ledger':
            return state, 'unsupported_independent_manual_contribution'
        rows[('manual', 'manual')] = contribution(bounded(int(cmd['value'])))
    elif kind == 'aggregate_set':
        target = bounded(int(cmd['value']))
        known = sum(row['amount'] for identity, row in rows.items() if identity[0] != 'legacy_adjustment')
        rows[('legacy_adjustment', 'unknown')] = contribution(bounded(target - known, 128))
    elif kind == 'adopt_exclusive':
        if scope['ownership'] != 'exclusive:' + cmd['identity']:
            return state, 'needs_reconciliation'
        value = bounded(int(cmd['value']))
        if total(scope) != value:
            return state, 'exclusive_totals_mismatch'
        scope['ownership'] = 'open'
        scope['rows'] = {('source', cmd['identity']): contribution(value, cmd['revision'])}
    else:
        raise RuntimeError('Unknown command kind ' + kind)
    total(scope)
    if kind == 'manual_add' and variant == 'aggregate_ledger':
        return state, 'applied_but_manual_attribution_not_stored'
    return state, 'applied'


class Store:
    def __init__(self, variant, path):
        self.variant = variant
        self.db = sqlite3.connect(path)
        self.db.row_factory = sqlite3.Row
        self.db.executescript(SCHEMAS[variant])

    def stored(self):
        tables = [r[0] for r in self.db.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")]
        return {name: [dict(row) for row in self.db.execute(f'SELECT * FROM {name} ORDER BY rowid')]
                for name in tables}

    def load(self):
        stored = self.stored()
        if self.variant == 'command_log':
            state = {}
            for row in stored['commands']:
                state, outcome = decide(state, json.loads(row['payload_json']), self.variant)
                if outcome != 'applied':
                    raise RuntimeError('Accepted command failed replay ' + outcome)
            return state
        state = {(r['metric'], r['day']): {'ownership': r['ownership'], 'rows': {}}
                 for r in stored['scopes']}
        if self.variant == 'contributor_rows':
            for row in stored['contributions']:
                state[(row['metric'], row['day'])]['rows'][(row['kind'], row['identity'])] = contribution(
                    row['amount_decimal'], row['revision'], row['cleared'])
        else:
            for row in stored['source_ledger']:
                state[(row['metric'], row['day'])]['rows'][('source', row['identity'])] = contribution(
                    row['value'], row['revision'], row['cleared'])
            for row in stored['aggregates']:
                rows = state[(row['metric'], row['day'])]['rows']
                residual = row['value'] - sum(r['amount'] for r in rows.values())
                rows[('legacy_adjustment', 'unknown')] = contribution(bounded(residual, 128))
        return state

    def apply(self, cmd):
        old_state = self.load()
        try:
            new_state, outcome = decide(old_state, cmd, self.variant)
        except ValueError as error:
            return str(error)
        if not outcome.startswith('applied'):
            return outcome
        if self.variant == 'command_log':
            self.db.execute('INSERT INTO commands(kind,payload_json) VALUES(?,?)',
                            (cmd['kind'], json.dumps(cmd, sort_keys=True)))
            return outcome
        for (metric, day), scope in new_state.items():
            old_scope = old_state.get((metric, day), {'rows': {}})
            if scope == old_scope:
                continue
            self.db.execute('INSERT INTO scopes VALUES(?,?,?) ON CONFLICT(metric,day) DO UPDATE SET ownership=excluded.ownership',
                            (metric, day, scope['ownership']))
            rows = scope['rows']
            if self.variant == 'aggregate_ledger':
                self.db.execute('INSERT INTO aggregates VALUES(?,?,?) ON CONFLICT(metric,day) DO UPDATE SET value=excluded.value',
                                (metric, day, total(scope)))
                for (kind, identity), row in rows.items():
                    if kind != 'source' or old_scope['rows'].get((kind, identity)) == row:
                        continue
                    self.db.execute('INSERT INTO source_ledger VALUES(?,?,?,?,?,?) ON CONFLICT(metric,day,identity) DO UPDATE SET value=excluded.value,revision=excluded.revision,cleared=excluded.cleared',
                                    (metric, day, identity, row['amount'], row['revision'], row['cleared']))
            else:
                for (kind, identity) in old_scope['rows'].keys() - rows.keys():
                    self.db.execute('DELETE FROM contributions WHERE metric=? AND day=? AND kind=? AND identity=?',
                                    (metric, day, kind, identity))
                for (kind, identity), row in rows.items():
                    if old_scope['rows'].get((kind, identity)) == row:
                        continue
                    self.db.execute('INSERT INTO contributions VALUES(?,?,?,?,?,?,?) ON CONFLICT(metric,day,kind,identity) DO UPDATE SET amount_decimal=excluded.amount_decimal,revision=excluded.revision,cleared=excluded.cleared',
                                    (metric, day, kind, identity, str(row['amount']), row['revision'], row['cleared']))
        return outcome

    def transaction(self, commands, fail_after=None):
        outcomes = []
        self.db.execute('BEGIN IMMEDIATE')
        try:
            for index, cmd in enumerate(commands):
                outcome = self.apply(cmd)
                outcomes.append(outcome)
                if not (outcome.startswith('applied') or outcome == 'unchanged'):
                    self.db.rollback()
                    return {'outcomes': outcomes, 'commit': False}
                if fail_after == index:
                    self.db.rollback()
                    return {'outcomes': outcomes, 'commit': False, 'injected_failure': 'after_old_day_clear'}
            self.db.commit()
            return {'outcomes': outcomes, 'commit': True}
        except Exception:
            self.db.rollback()
            raise

    def observe(self):
        views = []
        for (metric, day), scope in sorted(self.load().items()):
            views.append({'metric': metric, 'day': day, 'ownership': scope['ownership'],
                          'total': str(total(scope)),
                          'contributors': [{'kind': kind, 'identity': identity,
                                            'amount': str(row['amount']), 'revision': row['revision'],
                                            'cleared': row['cleared'],
                                            'storage': 'inferred_only' if self.variant == 'aggregate_ledger' and kind == 'legacy_adjustment' else 'computed_from_commands' if self.variant == 'command_log' else 'stored_row'}
                                           for (kind, identity), row in sorted(scope['rows'].items())]})
        return {'views': views, 'stored_rows': self.stored()}


def command(kind, day=D1, **payload):
    return {'kind': kind, 'metric': 'count', 'day': day, **payload}


def source(identity, value, revision=1, day=D1, **payload):
    return command('source_snapshot', day, identity=identity, value=str(value), revision=revision, **payload)


def seed(aggregate, known_sources=(), ownership='tracked_residual'):
    return command('legacy_seed', aggregate=str(aggregate), ownership=ownership,
                   known_sources=[{'identity': identity, 'value': str(value), 'revision': 1}
                                  for identity, value in known_sources])


def scenarios(variant, directory):
    records = []
    store = None

    def start(name):
        nonlocal store
        if store:
            store.db.close()
        store = Store(variant, directory / f'{variant}-{name}.sqlite')
        records.append({'name': name, 'steps': []})

    def step(label, *commands, fail_after=None):
        before = store.observe()
        outcome = store.transaction(commands, fail_after)
        after = store.observe()
        entry = {'label': label, 'commands': commands, **outcome, 'observation': after,
                 'storage_unchanged': before['stored_rows'] == after['stored_rows']}
        records[-1]['steps'].append(entry)
        totals = ','.join(v['day'] + '=' + v['total'] for v in after['views']) or 'empty'
        print(f"{variant} {records[-1]['name']}/{label}: {outcome['outcomes']} totals {totals}")

    start('source_identity_and_clear')
    step('A10_B5', source('A', 10), source('B', 5))
    step('A12_preserves_B', source('A', 12, 2))
    step('identical_snapshot', source('A', 12, 2))
    step('same_value_new_revision', source('A', 12, 3))
    step('same_revision_different_value', source('A', 13, 3))
    step('second_day', source('A', 7, day=D2))
    step('clear_A_omits_second_day', source('A', 0, 4, cleared=True))
    step('older_snapshot_after_clear', source('A', 12, 3))

    start('manual_add')
    step('A10_B5', source('A', 10), source('B', 5))
    step('add_manual_3', command('manual_add', value='3'))
    step('correct_A12', source('A', 12, 2))

    start('manual_contribution_set_10_4_12')
    step('source10', source('A', 10))
    step('set_manual_contribution4', command('manual_set', value='4'))
    step('source12', source('A', 12, 2))

    start('legacy_aggregate_set_10_4_12')
    step('source10', source('A', 10))
    step('set_aggregate4', command('aggregate_set', value='4'))
    step('source12', source('A', 12, 2))

    start('aggregate_set_preserves_known_manual')
    step('source10', source('A', 10))
    step('manual_add3', command('manual_add', value='3'))
    step('set_aggregate4', command('aggregate_set', value='4'))
    step('source12', source('A', 12, 2))

    start('negative_legacy_remainder')
    step('migrate_aggregate4_source10', seed(4, [('A', 10)]))
    step('source12', source('A', 12, 2))

    start('wide_legacy_remainder')
    step('migrate_MIN_source1', seed(MIN, [('A', 1)]))
    step('source1_replacement_final_fits', source('A', 1, 2))
    step('clear_would_overflow_final', source('A', 0, 3, cleared=True))

    start('mixed_baseline')
    step('opaque_total10', seed(10, ownership='mixed'))
    step('full_A_backfill10_refused', source('A', 10, full_backfill=True))
    step('exclusive_adoption_without_evidence_refused', command('adopt_exclusive', identity='A', value='10', revision=1))

    start('known_exclusive_adoption')
    step('known_A_exclusive_total10', seed(10, ownership='exclusive:A'))
    step('mismatched_adoption_refused', command('adopt_exclusive', identity='A', value='9', revision=1))
    step('adopt_matching_A10', command('adopt_exclusive', identity='A', value='10', revision=1))
    step('source12', source('A', 12, 2))

    start('source_record_move')
    step('record_old_day10_new_day_B5', source('A/record-1', 10), source('B', 5, day=D2))
    move = (source('A/record-1', 0, 2, cleared=True), source('A/record-1', 10, 2, day=D2))
    step('move_rollback', *move, fail_after=0)
    step('move_commit', *move)
    schema = [dict(row) for row in store.db.execute("SELECT type,name,sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY type,name")]
    store.db.close()
    return {'variant': variant, 'schema_from_sqlite': schema, 'scenarios': records}


def interval_observation(directory):
    db = sqlite3.connect(directory / 'intervals.sqlite')
    db.row_factory = sqlite3.Row
    db.execute('CREATE TABLE intervals(metric TEXT, source TEXT, day TEXT, start INTEGER, end INTEGER)')
    for metric in ('elapsed_union', 'effort_sum'):
        db.executemany('INSERT INTO intervals VALUES(?,?,?,?,?)',
                       [(metric, 'A', D1, 0, 10), (metric, 'B', D1, 5, 15)])
    db.commit()
    rows = [dict(row) for row in db.execute('SELECT * FROM intervals ORDER BY metric,source')]
    observed = {}
    for metric, policy in [('elapsed_union', 'union'), ('effort_sum', 'sum')]:
        spans = sorted((r['start'], r['end']) for r in rows if r['metric'] == metric)
        summed = sum(end - start for start, end in spans)
        union, right = 0, None
        for start, end in spans:
            union += max(0, end - max(start, right if right is not None else start))
            right = max(end, right if right is not None else end)
        observed[metric] = {'declared_policy': policy, 'sum': summed, 'union': union,
                            'selected_value': union if policy == 'union' else summed}
    schema = db.execute("SELECT sql FROM sqlite_master WHERE name='intervals'").fetchone()[0]
    db.close()
    print('interval rows: same overlap gives sum=20 and union=15; the metric chooses its policy')
    return {'schema': schema, 'stored_rows': rows, 'observed': observed,
            'limitation': 'Separate interval sketch, not an interval codec in the three integer models.'}


def encoding_observation(directory):
    db = sqlite3.connect(directory / 'integer-boundary.sqlite')
    db.execute('CREATE TABLE integer_probe(value INTEGER)')
    value = MIN - 1
    try:
        db.execute('INSERT INTO integer_probe VALUES(?)', (value,))
        native = {'outcome': 'inserted'}
    except OverflowError as error:
        native = {'outcome': 'unsupported', 'error_type': type(error).__name__, 'message': str(error)}
    db.execute('CREATE TABLE text_probe(value TEXT)')
    db.executemany('INSERT INTO text_probe VALUES(?)', [(str(value),), ('1',)])
    db.commit()
    sql_sum, sql_type = db.execute('SELECT SUM(value),typeof(SUM(value)) FROM text_probe').fetchone()
    result = {'residual_decimal': str(value), 'aggregate_decimal': str(MIN), 'source_decimal': '1',
              'bound_integer_insert': native,
              'text_rows': [r[0] for r in db.execute('SELECT value FROM text_probe ORDER BY rowid')],
              'sqlite_sum_text': {'value_repr': repr(sql_sum), 'sqlite_type': sql_type},
              'checked_application_sum': str(bounded(bounded(value + 1, 128))),
              'implication': 'A residual INTEGER row cannot encode MIN-1. Decimal TEXT requires exact application arithmetic; SQLite SUM coerces this example to REAL.'}
    db.close()
    print(f"i64 boundary: residual={value}, native bind={native['outcome']}, SQL SUM(TEXT) type={sql_type}")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--variant', choices=('all', *VARIANTS), default='all')
    args = parser.parse_args()
    selected = VARIANTS if args.variant == 'all' else (args.variant,)
    revision = subprocess.check_output(['git', '-C', str(REPO), 'rev-parse', 'HEAD'], text=True).strip()
    metadata = {'python': platform.python_version(), 'sqlite': sqlite3.sqlite_version,
                'source_revision_requested': SOURCE_REVISION, 'source_revision_observed': revision,
                'source_revision_matches': revision == SOURCE_REVISION,
                'command': f'python3 {ROOT / "run.py"} --variant {args.variant}',
                'numeric_boundary': 'Python arbitrary precision, explicitly checked i128 intermediates and i64 final totals, manual/source amounts. Contributor residuals are decimal TEXT. Not a Rust execution.',
                'data': 'synthetic only; temporary SQLite databases below storage/ removed at exit',
                'exit_policy': 'Nonzero only for runner failures. Unsupported operations and design limitations are observations.'}
    print(json.dumps(metadata, sort_keys=True))
    with tempfile.TemporaryDirectory(prefix='temporary-dbs-', dir=ROOT) as temp:
        directory = Path(temp)
        variants = [scenarios(variant, directory) for variant in selected]
        results = {'status': 'ISSUES', 'metadata': metadata, 'variants': variants,
                   'encoding_boundary': encoding_observation(directory),
                   'interval_policy': interval_observation(directory),
                   'issues': ['aggregate_ledger cannot independently set or persist a manual contribution without extra state',
                              'legacy residuals accepted by existing arithmetic can exceed an i64 contributor row',
                              'mixed baselines require reconciliation, not automatic full source backfill'],
                   'recommendation': 'Use typed contributor rows, an explicitly unknown legacy_adjustment, and an exact wide residual codec. Keep command receipts separate; full command history is not required by these observations.'}
    filename = 'results.json' if args.variant == 'all' else f'results-{args.variant}.json'
    path = ROOT / filename
    path.write_text(json.dumps(results, indent=2) + '\n')
    print('ISSUES. Comparison completed. Observations saved to ' + str(path))


if __name__ == '__main__':
    main()
