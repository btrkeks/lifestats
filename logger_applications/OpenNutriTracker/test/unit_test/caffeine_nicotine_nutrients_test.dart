import 'dart:collection';

import 'package:flutter_test/flutter_test.dart';
import 'package:hive_ce/hive.dart';
import 'package:opennutritracker/core/data/dbo/intake_dbo.dart';
import 'package:opennutritracker/core/data/dbo/intake_type_dbo.dart';
import 'package:opennutritracker/core/data/dbo/meal_dbo.dart';
import 'package:opennutritracker/core/data/dbo/meal_nutriments_dbo.dart';
import 'package:opennutritracker/core/domain/entity/intake_entity.dart';
import 'package:opennutritracker/core/domain/entity/intake_type_entity.dart';
import 'package:opennutritracker/core/domain/entity/tracked_day_entity.dart';
import 'package:opennutritracker/core/utils/csv_data_exporter.dart';
import 'package:opennutritracker/core/utils/off_micronutrient_repair.dart';
import 'package:opennutritracker/features/add_meal/data/dto/fdc/fdc_const.dart';
import 'package:opennutritracker/features/add_meal/data/dto/fdc/fdc_food_nutriment_dto.dart';
import 'package:opennutritracker/features/add_meal/data/dto/off/off_product_nutriments_dto.dart';
import 'package:opennutritracker/features/add_meal/domain/entity/meal_entity.dart';
import 'package:opennutritracker/features/add_meal/domain/entity/meal_nutriments_entity.dart';
import 'package:opennutritracker/features/diary/presentation/widgets/daily_nutrient_panel.dart';

/// Feeds a Hive adapter the field map a build before caffeine and nicotine
/// wrote: a field count, then (index, value) pairs.
class _QueueReader extends Fake implements BinaryReader {
  final Queue<Object?> _values;

  _QueueReader(Iterable<Object?> values) : _values = Queue.of(values);

  @override
  int readByte() => _values.removeFirst() as int;

  @override
  dynamic read([int? typeId]) => _values.removeFirst();
}

MealNutrimentsEntity _nutriments({double? caffeine, double? nicotine}) =>
    MealNutrimentsEntity(
      energyKcal100: 2,
      carbohydrates100: 0,
      fat100: 0,
      proteins100: 0,
      sugars100: null,
      saturatedFat100: null,
      fiber100: null,
      caffeine100: caffeine,
      nicotine100: nicotine,
    );

IntakeEntity _intake(double amount, MealNutrimentsEntity nutriments) =>
    IntakeEntity(
      id: 'i-$amount',
      unit: 'ml',
      amount: amount,
      type: IntakeTypeEntity.breakfast,
      dateTime: DateTime(2026, 10, 27, 8, 12),
      meal: MealEntity(
        code: null,
        name: 'Coffee',
        url: null,
        mealQuantity: '100',
        mealUnit: 'ml',
        servingQuantity: null,
        servingUnit: 'ml',
        servingSize: null,
        nutriments: nutriments,
        source: MealSourceEntity.custom,
      ),
    );

void main() {
  group('sources', () {
    test('Open Food Facts caffeine arrives in grams and is stored in mg', () {
      // Red Bull, 9002490100070: caffeine_100g 0.032, caffeine_unit "g".
      final entity = MealNutrimentsEntity.fromOffNutriments(
        OFFProductNutrimentsDTO(
          energy_kcal_100g: 46,
          carbohydrates_100g: 11,
          fat_100g: 0,
          proteins_100g: 0,
          sugars_100g: 11,
          saturated_fat_100g: 0,
          fiber_100g: 0,
          caffeine_100g: '0.032',
        ),
      );
      expect(entity.caffeine100, closeTo(32, 1e-9));
      expect(entity.nicotine100, isNull);
      expect(entity.hasMicronutrientData, isTrue);
    });

    test('FDC caffeine (nutrient 262) is already mg', () {
      expect(FDCConst.fdcCaffeineId, 1057);
      final entity = MealNutrimentsEntity.fromFDCNutriments([
        FDCFoodNutrimentDTO(nutrientId: FDCConst.fdcCaffeineId, amount: 40),
      ]);
      expect(entity.caffeine100, 40);
    });
  });

  group('storage', () {
    test('a nutriments record written before the new fields still loads', () {
      // Fields 0..23, the layout every existing intake and custom food has.
      final legacy = <Object?>[24];
      for (var i = 0; i < 24; i++) {
        legacy
          ..add(i)
          ..add(i == 0 ? 100.0 : null);
      }
      final dbo = MealNutrimentsDBOAdapter().read(_QueueReader(legacy));
      expect(dbo.energyKcal100, 100);
      expect(dbo.caffeine100, isNull);
      expect(dbo.nicotine100, isNull);
    });

    test(
      'CSV round-trips both fields, and an older CSV reads them as null',
      () {
        final intake = IntakeDBO(
          id: 'gum',
          unit: 'g',
          amount: 1.5,
          type: IntakeTypeDBO.snack,
          dateTime: DateTime(2026, 10, 27, 14, 3),
          meal: MealDBO(
            code: null,
            name: 'Nicotine gum 2 mg',
            brands: null,
            thumbnailImageUrl: null,
            mainImageUrl: null,
            url: null,
            mealQuantity: '1.5',
            mealUnit: 'g',
            servingQuantity: 1.5,
            servingUnit: 'g',
            servingSize: '1 piece',
            nutriments: MealNutrimentsDBO(
              energyKcal100: 200,
              carbohydrates100: null,
              fat100: null,
              proteins100: null,
              sugars100: null,
              saturatedFat100: null,
              fiber100: null,
              caffeine100: 0,
              nicotine100: 133.3,
            ),
            source: MealSourceDBO.custom,
          ),
        );
        final csv = CsvDataExporter.intakesToCsv([intake]);
        final parsed = CsvDataExporter.parseIntakesFromCsv(csv).single;
        expect(parsed.meal.nutriments.caffeine100, 0);
        expect(parsed.meal.nutriments.nicotine100, 133.3);

        final lines = csv.trim().split('\n');
        final header = lines.first.split(',');
        final cells = lines[1].split(',');
        final cut = header.indexOf('caffeine_per_100g');
        final older =
            '${header.take(cut).join(',')}\n${cells.take(cut).join(',')}\n';
        final fromOlder = CsvDataExporter.parseIntakesFromCsv(older).single;
        expect(fromOlder.meal.nutriments.caffeine100, isNull);
        expect(fromOlder.meal.nutriments.nicotine100, isNull);
      },
    );

    test('the Open Food Facts unit repair leaves caffeine unscaled', () {
      final meal = MealDBO(
        code: '1',
        name: 'Cola',
        brands: null,
        thumbnailImageUrl: null,
        mainImageUrl: null,
        url: null,
        mealQuantity: null,
        mealUnit: 'ml',
        servingQuantity: null,
        servingUnit: null,
        servingSize: null,
        nutriments: MealNutrimentsDBO(
          energyKcal100: 42,
          carbohydrates100: 10.6,
          fat100: 0,
          proteins100: 0,
          sugars100: 10.6,
          saturatedFat100: 0,
          fiber100: 0,
          sodium100: 0.01,
          caffeine100: 10,
        ),
        source: MealSourceDBO.off,
      );
      final repaired = OffMicronutrientRepair.repairMeal(meal);
      expect(repaired.nutriments.sodium100, closeTo(10, 1e-9));
      expect(repaired.nutriments.caffeine100, 10);
    });
  });

  group('daily panel', () {
    test('sums caffeine and nicotine from amount × per-100 value', () {
      final totals = NutrientPanelTotals.fromIntakes([
        // 200 ml coffee at 40 mg / 100 ml.
        _intake(200, _nutriments(caffeine: 40)),
        // 1.5 g gum at 133.3 mg / 100 g, i.e. 2 mg a piece.
        _intake(1.5, _nutriments(nicotine: 133.3)),
        // A food without either field adds nothing.
        _intake(300, _nutriments()),
      ]);
      expect(totals.caffeineMg, closeTo(80, 1e-9));
      expect(totals.nicotineMg, closeTo(2, 1e-3));
    });

    test(
      'caffeine defaults to the 400 mg EFSA limit unless the day sets one',
      () {
        expect(DailyNutrientPanel.resolveCaffeineReference(null), 400);
        final day = TrackedDayEntity(
          day: DateTime(2026, 10, 27),
          calorieGoal: 2000,
          caloriesTracked: 0,
          caffeineGoal: 200,
        );
        expect(DailyNutrientPanel.resolveCaffeineReference(day), 200);
      },
    );

    test('both rows have visibility keys', () {
      expect(NutrientPanelKeys.all, contains(NutrientPanelKeys.caffeine));
      expect(NutrientPanelKeys.all, contains(NutrientPanelKeys.nicotine));
    });
  });
}
