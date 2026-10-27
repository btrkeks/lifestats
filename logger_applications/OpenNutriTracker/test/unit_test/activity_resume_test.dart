import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:opennutritracker/core/utils/activity_resume.dart';

void main() {
  testWidgets('headless startup waits for an attached resumed activity', (
    tester,
  ) async {
    final binding = tester.binding;
    binding.handleAppLifecycleStateChanged(AppLifecycleState.detached);
    var started = false;
    final startup = waitForActivityResume().then((_) => started = true);
    await tester.pump();
    expect(started, isFalse);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await startup;
    expect(started, isTrue);
  });

  testWidgets('an activity already resumed proceeds immediately', (
    tester,
  ) async {
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await waitForActivityResume();
  });
}
