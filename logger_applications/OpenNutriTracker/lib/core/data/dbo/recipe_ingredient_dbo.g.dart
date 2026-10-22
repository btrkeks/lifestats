// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'recipe_ingredient_dbo.dart';

// **************************************************************************
// TypeAdapterGenerator
// **************************************************************************

class RecipeIngredientDBOAdapter extends TypeAdapter<RecipeIngredientDBO> {
  @override
  final typeId = 17;

  @override
  RecipeIngredientDBO read(BinaryReader reader) {
    final numOfFields = reader.readByte();
    final fields = <int, dynamic>{
      for (int i = 0; i < numOfFields; i++) reader.readByte(): reader.read(),
    };
    return RecipeIngredientDBO(
      snapshotMeal: fields[0] as MealDBO,
      amount: (fields[1] as num).toDouble(),
      unit: fields[2] as String,
      convertedAmountG: (fields[3] as num).toDouble(),
    );
  }

  @override
  void write(BinaryWriter writer, RecipeIngredientDBO obj) {
    writer
      ..writeByte(4)
      ..writeByte(0)
      ..write(obj.snapshotMeal)
      ..writeByte(1)
      ..write(obj.amount)
      ..writeByte(2)
      ..write(obj.unit)
      ..writeByte(3)
      ..write(obj.convertedAmountG);
  }

  @override
  int get hashCode => typeId.hashCode;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is RecipeIngredientDBOAdapter &&
          runtimeType == other.runtimeType &&
          typeId == other.typeId;
}

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

RecipeIngredientDBO _$RecipeIngredientDBOFromJson(Map<String, dynamic> json) =>
    RecipeIngredientDBO(
      snapshotMeal: MealDBO.fromJson(
        json['snapshotMeal'] as Map<String, dynamic>,
      ),
      amount: (json['amount'] as num).toDouble(),
      unit: json['unit'] as String,
      convertedAmountG: (json['convertedAmountG'] as num).toDouble(),
    );

Map<String, dynamic> _$RecipeIngredientDBOToJson(
  RecipeIngredientDBO instance,
) => <String, dynamic>{
  'snapshotMeal': instance.snapshotMeal,
  'amount': instance.amount,
  'unit': instance.unit,
  'convertedAmountG': instance.convertedAmountG,
};
