import 'package:hive_ce/hive.dart';
import 'package:opennutritracker/core/domain/entity/user_gender_entity.dart';

part 'user_gender_dbo.g.dart';

@HiveType(typeId: 6)
enum UserGenderDBO {
  @HiveField(0)
  male,
  @HiveField(1)
  female,
  @HiveField(2)
  nonBinary;

  factory UserGenderDBO.fromUserGenderEntity(UserGenderEntity genderEntity) {
    switch (genderEntity) {
      case UserGenderEntity.male:
        return UserGenderDBO.male;
      case UserGenderEntity.female:
        return UserGenderDBO.female;
      case UserGenderEntity.nonBinary:
        return UserGenderDBO.nonBinary;
    }
  }
}
