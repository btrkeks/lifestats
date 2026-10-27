import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/core/utils/calc/protein_goal_calc.dart';
import 'package:opennutritracker/features/home/presentation/bloc/home_bloc.dart';
import 'package:opennutritracker/features/settings/presentation/bloc/settings_bloc.dart';
import 'package:opennutritracker/features/settings/presentation/widgets/macro_split_dialog.dart';
import 'package:opennutritracker/generated/l10n.dart';
import '../../../../helpers/test_l10n.dart';

class _FakeSettingsBloc extends Fake implements SettingsBloc {
  int? savedCarbsSharePct;
  double? savedGramsPerKg;

  @override
  Future<double?> getUserCarbGoalPct() async => 0.6;

  @override
  Future<double?> getUserFatGoalPct() async => 0.25;

  @override
  Future<double?> getUserProteinGramsPerKg() async => null;

  @override
  Future<ProteinReferenceWeight> getProteinReferenceWeight() async =>
      const ProteinReferenceWeight(weightKg: 82.4, sampleCount: 4);

  @override
  Future<void> setMacroGoals({
    required int carbsSharePct,
    required double proteinGramsPerKg,
  }) async {
    savedCarbsSharePct = carbsSharePct;
    savedGramsPerKg = proteinGramsPerKg;
  }

  @override
  Future<void> updateTrackedDay(DateTime day) async {}

  @override
  void add(SettingsEvent event) {}
}

class _FakeHomeBloc extends Fake implements HomeBloc {
  @override
  void add(HomeEvent event) {}
}

Widget _wrap(Widget child) {
  return MaterialApp(
    localizationsDelegates: const [S.delegate],
    supportedLocales: S.supportedLocales,
    home: Scaffold(body: child),
  );
}

Future<_FakeSettingsBloc> _open(WidgetTester tester) async {
  final settingsBloc = _FakeSettingsBloc();
  await tester.pumpWidget(
    _wrap(
      Builder(
        builder: (context) {
          return ElevatedButton(
            onPressed: () => showDialog<void>(
              context: context,
              builder: (_) => MacroSplitDialog(
                settingsBloc: settingsBloc,
                homeBloc: _FakeHomeBloc(),
              ),
            ),
            child: const Text('Open'),
          );
        },
      ),
    ),
  );
  await tester.tap(find.text('Open'));
  await tester.pumpAndSettle();
  return settingsBloc;
}

void main() {
  testWidgets('loads the default factor and the 60:25 carbs/fat ratio', (
    tester,
  ) async {
    await _open(tester);

    final fields = find.byType(TextField);
    expect(tester.widget<TextField>(fields.at(0)).controller!.text, '1.8');
    expect(tester.widget<TextField>(fields.at(1)).controller!.text, '71');
    expect(find.text('29 %'), findsOneWidget);
    // 1.8 × 82.4 = 148.3 → 150 g.
    expect(
      find.text(l10nEn.settingsMacroSplitProteinPreview('150', '82.4')),
      findsOneWidget,
    );
  });

  testWidgets('saves a typed factor even without submitting the field', (
    tester,
  ) async {
    final settingsBloc = await _open(tester);

    await tester.enterText(find.byType(TextField).first, '2,2');
    await tester.tap(find.text(l10nEn.dialogOKLabel));
    await tester.pumpAndSettle();

    expect(settingsBloc.savedGramsPerKg, 2.2);
    expect(settingsBloc.savedCarbsSharePct, 71);
  });

  testWidgets('saves a typed carbs share', (tester) async {
    final settingsBloc = await _open(tester);

    await tester.enterText(find.byType(TextField).at(1), '60');
    await tester.tap(find.text(l10nEn.dialogOKLabel));
    await tester.pumpAndSettle();

    expect(settingsBloc.savedCarbsSharePct, 60);
    expect(settingsBloc.savedGramsPerKg, ProteinGoalCalc.defaultGramsPerKg);
  });

  testWidgets('ignores out-of-range typed values on save', (tester) async {
    final settingsBloc = await _open(tester);

    await tester.enterText(find.byType(TextField).first, '5');
    await tester.tap(find.text(l10nEn.dialogOKLabel));
    await tester.pumpAndSettle();

    expect(settingsBloc.savedGramsPerKg, ProteinGoalCalc.defaultGramsPerKg);
    expect(settingsBloc.savedCarbsSharePct, 71);
  });

  test('parseProteinGramsPerKg accepts both separators within range', () {
    expect(parseProteinGramsPerKg('1.6'), 1.6);
    expect(parseProteinGramsPerKg('2,2'), 2.2);
    expect(parseProteinGramsPerKg(' 1.84 '), 1.8);
    expect(parseProteinGramsPerKg('0.5'), isNull);
    expect(parseProteinGramsPerKg('3.5'), isNull);
    expect(parseProteinGramsPerKg(''), isNull);
  });
}
