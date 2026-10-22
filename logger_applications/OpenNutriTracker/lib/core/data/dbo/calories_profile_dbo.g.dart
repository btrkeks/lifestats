// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'calories_profile_dbo.dart';

// **************************************************************************
// TypeAdapterGenerator
// **************************************************************************

class CaloriesProfileDBOAdapter extends TypeAdapter<CaloriesProfileDBO> {
  @override
  final typeId = 18;

  @override
  CaloriesProfileDBO read(BinaryReader reader) {
    switch (reader.readByte()) {
      case 0:
        return CaloriesProfileDBO.averaged;
      case 1:
        return CaloriesProfileDBO.estrogenTypical;
      case 2:
        return CaloriesProfileDBO.testosteroneTypical;
      default:
        return CaloriesProfileDBO.averaged;
    }
  }

  @override
  void write(BinaryWriter writer, CaloriesProfileDBO obj) {
    switch (obj) {
      case CaloriesProfileDBO.averaged:
        writer.writeByte(0);
      case CaloriesProfileDBO.estrogenTypical:
        writer.writeByte(1);
      case CaloriesProfileDBO.testosteroneTypical:
        writer.writeByte(2);
    }
  }

  @override
  int get hashCode => typeId.hashCode;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is CaloriesProfileDBOAdapter &&
          runtimeType == other.runtimeType &&
          typeId == other.typeId;
}
