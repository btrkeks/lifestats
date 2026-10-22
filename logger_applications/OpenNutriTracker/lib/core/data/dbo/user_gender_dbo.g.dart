// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_gender_dbo.dart';

// **************************************************************************
// TypeAdapterGenerator
// **************************************************************************

class UserGenderDBOAdapter extends TypeAdapter<UserGenderDBO> {
  @override
  final typeId = 6;

  @override
  UserGenderDBO read(BinaryReader reader) {
    switch (reader.readByte()) {
      case 0:
        return UserGenderDBO.male;
      case 1:
        return UserGenderDBO.female;
      case 2:
        return UserGenderDBO.nonBinary;
      default:
        return UserGenderDBO.male;
    }
  }

  @override
  void write(BinaryWriter writer, UserGenderDBO obj) {
    switch (obj) {
      case UserGenderDBO.male:
        writer.writeByte(0);
      case UserGenderDBO.female:
        writer.writeByte(1);
      case UserGenderDBO.nonBinary:
        writer.writeByte(2);
    }
  }

  @override
  int get hashCode => typeId.hashCode;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is UserGenderDBOAdapter &&
          runtimeType == other.runtimeType &&
          typeId == other.typeId;
}
