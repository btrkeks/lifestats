import 'dart:async';
import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:hive_ce/hive.dart';
import 'package:logging/logging.dart';
import 'package:opennutritracker/core/utils/hive_db_provider.dart';
import 'package:opennutritracker/features/quick_log/domain/quick_log_command.dart';
import 'package:opennutritracker/features/quick_log/quick_log_service.dart';
import 'package:synchronized/synchronized.dart';

/// The Dart end of `com.opennutritracker/quick_log`, registered on the
/// process engine by Kotlin's `QuickLogBridge`.
///
///   Kotlin -> Dart   run(Map command) -> Map result
///   Dart -> Kotlin   publish(String json): replace the widget snapshot
///
/// Kotlin waits for `EngineReadiness` before sending, so [install] must run
/// before `signalReady`.
class QuickLogBridge {
  static const channel = MethodChannel('com.opennutritracker/quick_log');
  static final _log = Logger('QuickLogBridge');

  final QuickLogService _service;
  final HiveDBProvider _db;

  /// One command or publish at a time, in arrival order, so a reply always
  /// follows the snapshot that includes its write.
  final Lock _serial = Lock();
  QuickLogPublisher? _publisher;

  QuickLogBridge(this._service, this._db);

  void install() {
    channel.setMethodCallHandler(_onCall);
    _publisher ??= QuickLogPublisher(_db, publish)..start();
    unawaited(publish());
  }

  void dispose() {
    channel.setMethodCallHandler(null);
    _publisher?.stop();
    _publisher = null;
  }

  Future<void> publish() => _serial.synchronized(_publishNow);

  Future<Object?> _onCall(MethodCall call) async {
    if (call.method != 'run') throw MissingPluginException();
    final command = QuickLogCommand.fromWire(call.arguments);
    if (command == null) {
      return const QuickLogRejected(RejectReason.invalid).toWire();
    }
    return _serial.synchronized(() async {
      final result = await _service.run(command);
      await _publishNow();
      return result.toWire();
    });
  }

  /// Never throws: a snapshot that fails to build must not turn a committed
  /// write into an error reply.
  Future<void> _publishNow() async {
    try {
      final snapshot = await _service.snapshot();
      await channel.invokeMethod<void>(
        'publish',
        jsonEncode(snapshot.toJson()),
      );
    } on MissingPluginException {
      // No widget host: iOS, or a test.
    } catch (error, stackTrace) {
      _log.warning('publish failed', error, stackTrace);
    }
  }
}

/// Republishes the widget snapshot whenever something it is built from
/// changes, whoever wrote it: the app has many writers, and hooking each
/// one would be a list kept in sync by hand.
class QuickLogPublisher {
  final HiveDBProvider _db;
  final Future<void> Function() _publish;
  final Duration debounce;

  final List<StreamSubscription<BoxEvent>> _subscriptions = [];
  Timer? _pending;

  QuickLogPublisher(
    this._db,
    this._publish, {
    this.debounce = const Duration(milliseconds: 300),
  });

  void start() {
    _db.addListener(_onProfileSwitched);
    _subscribe();
  }

  void stop() {
    _db.removeListener(_onProfileSwitched);
    _unsubscribe();
    _pending?.cancel();
  }

  /// A profile switch closes the old boxes, which ends their streams.
  void _onProfileSwitched() {
    _subscribe();
    _schedule();
  }

  void _subscribe() {
    _unsubscribe();
    for (final box in <BoxBase<Object?>>[
      _db.intakeBox,
      _db.configBox,
      _db.userBox,
      _db.userActivityBox,
      _db.appConfigBox,
    ]) {
      _subscriptions.add(box.watch().listen((_) => _schedule()));
    }
  }

  void _unsubscribe() {
    for (final subscription in _subscriptions) {
      unawaited(subscription.cancel());
    }
    _subscriptions.clear();
  }

  void _schedule() {
    _pending?.cancel();
    _pending = Timer(debounce, () => unawaited(_publish()));
  }
}
