import 'package:opennutritracker/core/data/repository/water_intake_repository.dart';

class DeleteWaterIntakeUsecase {
  final WaterIntakeRepository _waterIntakeRepository;

  DeleteWaterIntakeUsecase(this._waterIntakeRepository);

  Future<void> deleteEntry(String id) async {
    await _waterIntakeRepository.deleteEntry(id);
  }
}
