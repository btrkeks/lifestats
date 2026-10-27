import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/core/utils/launch_request_service.dart';

// #927: Health Connect asks the app to explain what it wants health data for
// by starting it with one of two intents. The manifest declared both and
// `MainActivity` ignored them, so the question was answered by dropping the
// user on the diary. The channel tests pin the Dart contract; the source
// tests pin the halves that live outside Dart and cannot fail a compile.

const _channel = MethodChannel('com.opennutritracker/launch');

/// The two actions, spelled exactly as Android delivers them. Duplicated here
/// on purpose — a test that read them from the source it checks would agree
/// with any typo.
const _rationaleAction = 'androidx.health.ACTION_SHOW_PERMISSIONS_RATIONALE';
const _permissionUsageAction = 'android.intent.action.VIEW_PERMISSION_USAGE';

const _kotlinDir =
    'android/app/src/main/kotlin/com/opennutritracker/ont/opennutritracker';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  final calls = <MethodCall>[];
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;

  void handleWith(Future<Object?>? Function(MethodCall)? handler) {
    messenger.setMockMethodCallHandler(_channel, (call) async {
      calls.add(call);
      return handler == null ? null : await handler(call);
    });
  }

  setUp(calls.clear);
  tearDown(() => messenger.setMockMethodCallHandler(_channel, null));

  group('LaunchRequestService', () {
    test('reports a pending health rationale request', () async {
      handleWith((_) async => 'healthRationale');

      expect(
        await LaunchRequestService.consume(),
        LaunchRequest.healthRationale,
      );
      expect(calls.single.method, 'consume');
    });

    test('reports the widget\'s scan and search requests', () async {
      for (final request in [
        LaunchRequest.scanProduct,
        LaunchRequest.searchFood,
      ]) {
        handleWith((_) async => request.name);

        expect(await LaunchRequestService.consume(), request);
      }
    });

    test('reports nothing pending on an ordinary launch', () async {
      handleWith((_) async => null);

      expect(await LaunchRequestService.consume(), isNull);
    });

    // A newer native side may record a request this build does not know.
    // Acting on a guess would put people on a screen they never asked for.
    test('ignores a request it does not recognise', () async {
      handleWith((_) async => 'somethingNew');

      expect(await LaunchRequestService.consume(), isNull);
    });

    // iOS, desktop and every widget test: no native side is registered. The
    // call must answer rather than throw into whatever is driving it.
    test('answers nothing on a platform with no native side', () async {
      expect(await LaunchRequestService.consume(), isNull);
    });

    test('answers nothing when the platform side fails', () async {
      handleWith((_) async => throw PlatformException(code: 'error'));

      expect(await LaunchRequestService.consume(), isNull);
    });
  });

  // These are the checks that would have caught the original bug: the manifest
  // was right and unread, which no Dart test could have noticed.
  group('the Android side is wired to answer', () {
    final manifest = File(
      'android/app/src/main/AndroidManifest.xml',
    ).readAsStringSync();
    final activity = File('$_kotlinDir/MainActivity.kt').readAsStringSync();
    final inbox = File('$_kotlinDir/LaunchInbox.kt').readAsStringSync();

    test('the manifest offers both ways in', () {
      expect(manifest, contains(_rationaleAction));
      expect(manifest, contains(_permissionUsageAction));
    });

    test('the inbox reads both actions the manifest advertises', () {
      // Declaring a filter nothing handles is precisely the bug in #927: the
      // user reaches the app and no question gets answered.
      expect(
        inbox,
        contains(_rationaleAction),
        reason:
            'manifest advertises the rationale action; LaunchInbox must '
            'recognise it or the user lands on the diary',
      );
      expect(
        inbox,
        contains(_permissionUsageAction),
        reason:
            'manifest advertises the permission-usage action; '
            'LaunchInbox must recognise it',
      );
      expect(inbox, contains('"${LaunchRequest.healthRationale.name}"'));
    });

    test('the inbox can record every request Dart acts on', () {
      for (final request in LaunchRequest.values) {
        expect(inbox, contains('"${request.name}"'), reason: request.name);
      }
    });

    test('the activity hands the inbox both kinds of launch', () {
      expect(activity, contains('LaunchInbox.offer(intent)'));
    });

    // singleTop means a running process is handed the intent through
    // onNewIntent. Reading only the launching intent works once per process
    // and then silently stops, which is a worse failure than the original
    // because it passes a first manual test.
    test('the activity survives being reached twice', () {
      expect(
        manifest,
        contains('android:launchMode="singleTop"'),
        reason:
            'the onNewIntent override below is only needed while this '
            'holds — if the launch mode changed, re-check the activity',
      );
      expect(activity, contains('onNewIntent'));
    });

    // Without the permission guard any app on the device could open this
    // activity; Google's own snippet carries it.
    test('the permission-usage alias stays guarded', () {
      expect(
        manifest,
        contains(
          'android:permission="android.permission.START_VIEW_PERMISSION_USAGE"',
        ),
      );
    });
  });
}
