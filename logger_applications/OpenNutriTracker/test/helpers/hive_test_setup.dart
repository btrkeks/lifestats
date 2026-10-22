import 'package:hive_ce/hive.dart';
import 'package:opennutritracker/hive_registrar.g.dart';

/// Process-wide flag — Hive's TypeRegistry is shared across all tests in
/// the same `flutter test` run, so calling `Hive.registerAdapter(...)`
/// twice with the same typeId throws. This guard makes the call
/// idempotent so test files can use it freely from setUp.
bool _hiveAdaptersRegistered = false;

/// Registers every DBO adapter via the generated [HiveRegistrar] extension
/// in `hive_registrar.g.dart`. Safe to call from any test's setUp — the
/// first call registers, subsequent calls are no-ops.
///
/// Prefer this over hand-listing the adapters a specific test "needs":
/// the hand-list silently rots when new DBO types are added (the
/// CaloriesProfileDBO outage on develop was exactly that bug class), and
/// you only learn about it when something throws at write time.
void registerHiveAdaptersOnce() {
  if (_hiveAdaptersRegistered) return;
  Hive.registerAdapters();
  _hiveAdaptersRegistered = true;
}
