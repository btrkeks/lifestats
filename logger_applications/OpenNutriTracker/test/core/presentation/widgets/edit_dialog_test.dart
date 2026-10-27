import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/core/domain/entity/intake_entity.dart';
import 'package:opennutritracker/core/domain/entity/intake_type_entity.dart';
import 'package:opennutritracker/core/presentation/widgets/edit_dialog.dart';
import 'package:opennutritracker/features/add_meal/domain/entity/meal_entity.dart';
import 'package:opennutritracker/features/add_meal/domain/entity/meal_nutriments_entity.dart';
import 'package:opennutritracker/generated/l10n.dart';

IntakeEntity _coffee(DateTime dateTime) => IntakeEntity(
  id: 'coffee',
  unit: 'ml',
  amount: 200,
  type: IntakeTypeEntity.breakfast,
  dateTime: dateTime,
  meal: MealEntity(
    code: null,
    name: 'Coffee (my machine)',
    url: null,
    mealQuantity: '100',
    mealUnit: 'ml',
    servingQuantity: null,
    servingUnit: 'ml',
    servingSize: null,
    nutriments: const MealNutrimentsEntity(
      energyKcal100: 2,
      carbohydrates100: 0,
      fat100: 0,
      proteins100: 0,
      sugars100: null,
      saturatedFat100: null,
      fiber100: null,
      caffeine100: 40,
    ),
    source: MealSourceEntity.custom,
  ),
);

/// The time field shows the stored time and costs nothing to keep.
void main() {
  Future<EditIntakeResult?> confirm(
    WidgetTester tester,
    IntakeEntity intake,
  ) async {
    EditIntakeResult? result;
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: const [S.delegate],
        supportedLocales: S.supportedLocales,
        builder: (context, child) => MediaQuery(
          data: MediaQuery.of(context).copyWith(alwaysUse24HourFormat: true),
          child: child!,
        ),
        home: Builder(
          builder: (context) => Scaffold(
            body: ElevatedButton(
              onPressed: () async {
                result = await showDialog<EditIntakeResult>(
                  context: context,
                  builder: (_) => EditDialog(
                    intakeEntity: intake,
                    usesImperialUnits: false,
                  ),
                );
              },
              child: const Text('open'),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('OK'));
    await tester.pumpAndSettle();
    return result;
  }

  testWidgets('shows the stored time and returns no time when kept', (
    tester,
  ) async {
    final result = await confirm(
      tester,
      _coffee(DateTime(2026, 10, 27, 8, 12, 30)),
    );
    expect(result, isNotNull);
    expect(result!.amount, 200);
    expect(result.time, isNull);
  });

  testWidgets('an intake that only knows its day says so', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: const [S.delegate],
        supportedLocales: S.supportedLocales,
        home: Scaffold(
          body: EditDialog(
            intakeEntity: _coffee(DateTime.utc(2026, 10, 20)),
            usesImperialUnits: false,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Not recorded'), findsOneWidget);
  });

  testWidgets('the time field shows HH:MM', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: const [S.delegate],
        supportedLocales: S.supportedLocales,
        builder: (context, child) => MediaQuery(
          data: MediaQuery.of(context).copyWith(alwaysUse24HourFormat: true),
          child: child!,
        ),
        home: Scaffold(
          body: EditDialog(
            intakeEntity: _coffee(DateTime(2026, 10, 27, 8, 12, 30)),
            usesImperialUnits: false,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('08:12'), findsOneWidget);
  });
}
