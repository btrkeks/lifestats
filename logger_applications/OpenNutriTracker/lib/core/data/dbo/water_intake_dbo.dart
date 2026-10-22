import 'package:hive_ce_flutter/hive_flutter.dart';
import 'package:json_annotation/json_annotation.dart';
import 'package:opennutritracker/core/domain/entity/water_intake_entity.dart';

part 'water_intake_dbo.g.dart';

@HiveType(typeId: 19)
@JsonSerializable()
class WaterIntakeDBO extends HiveObject {
  @HiveField(0)
  String id;
  @HiveField(1)
  DateTime dateTime;
  @HiveField(2)
  int amountMl;

  WaterIntakeDBO({
    required this.id,
    required this.dateTime,
    required this.amountMl,
  });

  factory WaterIntakeDBO.fromWaterIntakeEntity(WaterIntakeEntity entity) {
    return WaterIntakeDBO(
      id: entity.id,
      dateTime: entity.dateTime,
      amountMl: entity.amountMl,
    );
  }

  factory WaterIntakeDBO.fromJson(Map<String, dynamic> json) =>
      _$WaterIntakeDBOFromJson(json);

  Map<String, dynamic> toJson() => _$WaterIntakeDBOToJson(this);
}
