import 'package:opennutritracker/core/data/repository/water_intake_repository.dart';
import 'package:opennutritracker/core/domain/entity/water_intake_entity.dart';

class AddWaterIntakeUsecase {
  final WaterIntakeRepository _waterIntakeRepository;

  AddWaterIntakeUsecase(this._waterIntakeRepository);

  Future<void> addEntry(WaterIntakeEntity entry) async {
    await _waterIntakeRepository.addEntry(entry);
  }
}
