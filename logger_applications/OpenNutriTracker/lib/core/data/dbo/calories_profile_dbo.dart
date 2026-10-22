import 'package:hive_ce/hive.dart';
import 'package:opennutritracker/core/domain/entity/calories_profile_entity.dart';

part 'calories_profile_dbo.g.dart';

@HiveType(typeId: 18)
enum CaloriesProfileDBO {
  @HiveField(0)
  averaged,
  @HiveField(1)
  estrogenTypical,
  @HiveField(2)
  testosteroneTypical;

  factory CaloriesProfileDBO.fromEntity(CaloriesProfileEntity entity) {
    switch (entity) {
      case CaloriesProfileEntity.averaged:
        return CaloriesProfileDBO.averaged;
      case CaloriesProfileEntity.estrogenTypical:
        return CaloriesProfileDBO.estrogenTypical;
      case CaloriesProfileEntity.testosteroneTypical:
        return CaloriesProfileDBO.testosteroneTypical;
    }
  }
}
