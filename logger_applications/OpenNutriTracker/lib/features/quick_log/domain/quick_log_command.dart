import 'package:opennutritracker/features/quick_log/domain/food_key.dart';

/// What the widget asks Dart to do, parsed once from the channel map.
///
/// Wire form (Kotlin `QuickLogCommand.toWire`):
///   {op: 'log', opId, foodKey, profileId, baseAmount, unit, tappedAtMs}
///   {op: 'undo', opId, profileId}
///   {op: 'refresh'}
sealed class QuickLogCommand {
  const QuickLogCommand();

  /// Null for anything malformed: an unknown op, a missing or mistyped
  /// field, an unparseable food key or a non-finite amount.
  static QuickLogCommand? fromWire(Object? arguments) {
    if (arguments is! Map) return null;
    final map = arguments.cast<Object?, Object?>();
    String? text(String key) {
      final value = map[key];
      return value is String && value.isNotEmpty ? value : null;
    }

    switch (map['op']) {
      case 'log':
        final opId = text('opId');
        final foodKey = FoodKey.parse(text('foodKey'));
        final profileId = text('profileId');
        final amount = map['baseAmount'];
        final unit = text('unit');
        final tappedAtMs = map['tappedAtMs'];
        if (opId == null ||
            foodKey == null ||
            profileId == null ||
            amount is! num ||
            !amount.isFinite ||
            unit == null ||
            tappedAtMs is! int) {
          return null;
        }
        return LogCommand(
          opId: opId,
          foodKey: foodKey,
          profileId: profileId,
          baseAmount: amount.toDouble(),
          unit: unit,
          tappedAt: DateTime.fromMillisecondsSinceEpoch(tappedAtMs),
        );
      case 'undo':
        final opId = text('opId');
        final profileId = text('profileId');
        if (opId == null || profileId == null) return null;
        return UndoCommand(opId: opId, profileId: profileId);
      case 'refresh':
        return const RefreshCommand();
      default:
        return null;
    }
  }
}

/// A pill tap. [opId] becomes the intake id, so replaying it writes nothing.
final class LogCommand extends QuickLogCommand {
  final String opId;
  final FoodKey foodKey;

  /// The profile the tapped snapshot was rendered for.
  final String profileId;
  final double baseAmount;
  final String unit;

  /// When the user tapped, not when Dart got to it: a cold boot can take
  /// seconds, and the meal type and day follow the tap.
  final DateTime tappedAt;

  const LogCommand({
    required this.opId,
    required this.foodKey,
    required this.profileId,
    required this.baseAmount,
    required this.unit,
    required this.tappedAt,
  });
}

final class UndoCommand extends QuickLogCommand {
  final String opId;
  final String profileId;

  const UndoCommand({required this.opId, required this.profileId});
}

/// Recompute and publish the snapshot; writes nothing.
final class RefreshCommand extends QuickLogCommand {
  const RefreshCommand();
}

sealed class QuickLogResult {
  const QuickLogResult();

  Map<String, Object?> toWire();
}

final class QuickLogOk extends QuickLogResult {
  /// "Added · +30 g" for a log, null otherwise.
  final String? addedText;

  const QuickLogOk({this.addedText});

  @override
  Map<String, Object?> toWire() => {'status': 'ok', 'addedText': addedText};
}

enum RejectReason { invalid, staleProfile, unknownFood, notSetUp }

final class QuickLogRejected extends QuickLogResult {
  final RejectReason reason;

  const QuickLogRejected(this.reason);

  @override
  Map<String, Object?> toWire() => {
    'status': 'rejected',
    'reason': reason.name,
  };
}
