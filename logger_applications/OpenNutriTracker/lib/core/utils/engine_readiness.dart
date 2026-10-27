import 'package:flutter/services.dart';

/// Tells the Android side (`OntEngine`) whether storage opened.
///
/// The engine can be started with no activity attached, and native code that
/// wants Dart to touch storage must wait until `initLocator` has opened it.
/// Only Android registers the other end; elsewhere the calls do nothing.
class EngineReadiness {
  static const _channel = MethodChannel('com.opennutritracker/engine');

  /// Storage is open and Dart can take commands.
  static Future<void> signalReady() => _send('ready');

  /// Storage failed to open, for [reason]. Not terminal: a later
  /// [signalReady] replaces it when a retry succeeds.
  static Future<void> signalUnavailable(String reason) =>
      _send('unavailable', reason);

  static Future<void> _send(String method, [Object? arguments]) async {
    try {
      await _channel.invokeMethod<void>(method, arguments);
    } on PlatformException {
      // Nothing native is waiting on an answer it failed to receive.
    } on MissingPluginException {
      // No native side registered: iOS, desktop, or a widget test.
    }
  }
}
