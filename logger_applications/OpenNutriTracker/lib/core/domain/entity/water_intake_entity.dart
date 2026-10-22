import 'package:equatable/equatable.dart';
import 'package:opennutritracker/core/data/dbo/water_intake_dbo.dart';

class WaterIntakeEntity extends Equatable {
  final String id;
  final DateTime dateTime;
  final int amountMl;

  const WaterIntakeEntity({
    required this.id,
    required this.dateTime,
    required this.amountMl,
  });

  factory WaterIntakeEntity.fromWaterIntakeDBO(WaterIntakeDBO dbo) {
    return WaterIntakeEntity(
      id: dbo.id,
      dateTime: dbo.dateTime,
      amountMl: dbo.amountMl,
    );
  }

  @override
  List<Object?> get props => [id, dateTime, amountMl];
}
