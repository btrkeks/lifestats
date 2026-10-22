import 'package:opennutritracker/core/data/data_source/water_intake_data_source.dart';
import 'package:opennutritracker/core/data/dbo/water_intake_dbo.dart';
import 'package:opennutritracker/core/domain/entity/water_intake_entity.dart';

class WaterIntakeRepository {
  final WaterIntakeDataSource _waterIntakeDataSource;

  WaterIntakeRepository(this._waterIntakeDataSource);

  Future<void> addEntry(WaterIntakeEntity entry) async {
    await _waterIntakeDataSource.addEntry(
      WaterIntakeDBO.fromWaterIntakeEntity(entry),
    );
  }

  Future<void> addAllEntries(List<WaterIntakeDBO> entries) async {
    await _waterIntakeDataSource.addAllEntries(entries);
  }

  Future<List<WaterIntakeEntity>> getAllEntries() async {
    final dbos = await _waterIntakeDataSource.allEntries();
    return dbos.map(WaterIntakeEntity.fromWaterIntakeDBO).toList();
  }

  Future<List<WaterIntakeDBO>> getAllEntriesDBO() async {
    return _waterIntakeDataSource.allEntries();
  }

  Future<List<WaterIntakeEntity>> getEntriesInRange(
    DateTime from,
    DateTime to,
  ) async {
    final dbos = await _waterIntakeDataSource.entriesInRange(from, to);
    return dbos.map(WaterIntakeEntity.fromWaterIntakeDBO).toList();
  }

  Future<void> deleteEntry(String id) async {
    await _waterIntakeDataSource.deleteEntry(id);
  }
}
