import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/core/domain/entity/intake_entity.dart';
import 'package:opennutritracker/features/quick_log/domain/suggestion_ranking.dart';
import 'quick_log_test_data.dart';

void main() {
  final today = DateTime(2026, 10, 27);
  List<Suggestion> rank(List<IntakeEntity> rows, {int offset = 0}) =>
      rankSuggestions(
        intakes: rows,
        today: today,
        dayStartOffsetMinutes: offset,
      );
  List<String> codes(List<Suggestion> rows) =>
      rows.map((s) => s.latest.meal.code!).toList();

  test('D-7 included, D-8 and future days excluded', () {
    expect(
      codes(
        rank([
          intake('edge', DateTime(2026, 10, 20, 12), code: 'edge'),
          intake('old', DateTime(2026, 10, 19, 23), code: 'old'),
          intake('future', DateTime(2026, 10, 28, 12), code: 'future'),
        ]),
      ),
      ['edge'],
    );
  });
  test('04:30 boundary at 02:00 uses the preceding logical day', () {
    expect(
      codes(
        rank([
          intake('inside', DateTime(2026, 10, 21, 2), code: 'inside'),
          intake('outside', DateTime(2026, 10, 20, 2), code: 'outside'),
        ], offset: 270),
      ),
      ['inside'],
    );
  });
  test('midnight local and UTC imports retain their date label like Home', () {
    final result = codes(
      rank([
        intake('local', DateTime(2026, 10, 20), code: 'local'),
        intake('utc', DateTime.utc(2026, 10, 20), code: 'utc'),
        intake('today', today, code: 'today'),
      ], offset: 270),
    );
    expect(result.take(2), unorderedEquals(['local', 'utc']));
    expect(result.last, 'today');
  });
  test('counts then latest historical date then key determine the head', () {
    final rows = [
      intake('a', DateTime(2026, 10, 26, 12), code: 'a'),
      intake('b', DateTime(2026, 10, 26, 12), code: 'b'),
      intake('c', DateTime(2026, 10, 26, 13), code: 'c'),
      intake('d1', DateTime(2026, 10, 20, 12), code: 'd'),
      intake('d2', DateTime(2026, 10, 21, 12), code: 'd'),
    ];
    expect(codes(rank(rows.reversed.toList())), ['d', 'c', 'a', 'b']);
  });
  test('today updates quick amount without moving the historical head', () {
    final rows = [
      intake('a', DateTime(2026, 10, 26, 12), code: 'a'),
      intake('b', DateTime(2026, 10, 25, 12), code: 'b'),
      for (var i = 0; i < 5; i++)
        intake(
          'b$i',
          DateTime(2026, 10, 27, 12, i),
          code: 'b',
          amount: 100 + i.toDouble(),
        ),
    ];
    final result = rank(rows);
    expect(codes(result), ['a', 'b']);
    expect(result.last.latest.amount, 104);
  });
  test('today-only tail is ordered by first log, not latest log or count', () {
    expect(
      codes(
        rank([
          intake('b1', DateTime(2026, 10, 27, 10), code: 'b'),
          intake('a1', DateTime(2026, 10, 27, 9), code: 'a'),
          intake('a2', DateTime(2026, 10, 27, 13), code: 'a'),
          intake('hist', DateTime(2026, 10, 26, 12), code: 'hist'),
        ]),
      ),
      ['hist', 'a', 'b'],
    );
  });
  test('recent amounts are distinct pairs, newest first, capped at four', () {
    final result = rank([
      for (var i = 0; i < 6; i++)
        intake('$i', DateTime(2026, 10, 27, 10, i), amount: 100 + i.toDouble()),
      intake('duplicate', DateTime(2026, 10, 27, 11), amount: 105),
      intake(
        'other-unit',
        DateTime(2026, 10, 27, 12),
        amount: 105,
        unit: 'serving',
      ),
    ]).single;
    expect(result.recentAmounts.map((i) => (i.amount, i.unit)), [
      (105.0, 'serving'),
      (105.0, 'g'),
      (104.0, 'g'),
      (103.0, 'g'),
    ]);
  });
  test('empty and unidentifiable histories give no suggestions', () {
    expect(rank([]), isEmpty);
    expect(rank([intake('blank', today, code: '', name: ' ')]), isEmpty);
  });
}
