import 'package:flutter/services.dart';

/// What the OS opened the app to do.
enum LaunchRequest {
  /// Health Connect asked the app to explain what it wants health data for
  /// (#927). It starts the app rather than rendering anything itself, so
  /// without an answer the user is dropped on the diary having asked a
  /// question nothing answered.
  healthRationale,
}

/// Reads the launch the platform recorded and has not handed over yet.
///
/// The Android side (`LaunchInbox`) records the intent in onCreate or
/// onNewIntent and holds it until this asks, because the widget tree that has
/// to react is not ready when the intent arrives. It is process-scoped, so it
/// answers whether the engine was started by the activity or by something
/// with no activity attached.
///
/// Only Android registers the other end of this channel. On iOS, the Linux
/// desktop build and every widget test the call raises
/// [MissingPluginException] and is answered as "nothing pending", which is the
/// truth on a platform with nothing to launch the app this way. Mirrors
/// `AppLocaleService`: same shape, same reason for having no
/// `Platform.isAndroid` guard.
class LaunchRequestService {
  static const _channel = MethodChannel('com.opennutritracker/launch');

  /// The pending request, or null. Reading clears it on the platform side,
  /// so two callers racing cannot both act on it.
  static Future<LaunchRequest?> consume() async {
    try {
      final wire = await _channel.invokeMethod<String>('consume');
      return LaunchRequest.values.asNameMap()[wire];
    } on PlatformException {
      return null;
    } on MissingPluginException {
      return null;
    }
  }
}
