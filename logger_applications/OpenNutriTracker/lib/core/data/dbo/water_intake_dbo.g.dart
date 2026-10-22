// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'water_intake_dbo.dart';

// **************************************************************************
// TypeAdapterGenerator
// **************************************************************************

class WaterIntakeDBOAdapter extends TypeAdapter<WaterIntakeDBO> {
  @override
  final typeId = 19;

  @override
  WaterIntakeDBO read(BinaryReader reader) {
    final numOfFields = reader.readByte();
    final fields = <int, dynamic>{
      for (int i = 0; i < numOfFields; i++) reader.readByte(): reader.read(),
    };
    return WaterIntakeDBO(
      id: fields[0] as String,
      dateTime: fields[1] as DateTime,
      amountMl: (fields[2] as num).toInt(),
    );
  }

  @override
  void write(BinaryWriter writer, WaterIntakeDBO obj) {
    writer
      ..writeByte(3)
      ..writeByte(0)
      ..write(obj.id)
      ..writeByte(1)
      ..write(obj.dateTime)
      ..writeByte(2)
      ..write(obj.amountMl);
  }

  @override
  int get hashCode => typeId.hashCode;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is WaterIntakeDBOAdapter &&
          runtimeType == other.runtimeType &&
          typeId == other.typeId;
}

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

WaterIntakeDBO _$WaterIntakeDBOFromJson(Map<String, dynamic> json) =>
    WaterIntakeDBO(
      id: json['id'] as String,
      dateTime: DateTime.parse(json['dateTime'] as String),
      amountMl: (json['amountMl'] as num).toInt(),
    );

Map<String, dynamic> _$WaterIntakeDBOToJson(WaterIntakeDBO instance) =>
    <String, dynamic>{
      'id': instance.id,
      'dateTime': instance.dateTime.toIso8601String(),
      'amountMl': instance.amountMl,
    };
