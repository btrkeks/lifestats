import 'dart:async';

import 'package:flutter/widgets.dart';

/// Storage can serve widget commands before any activity owns a view.
Future<void> waitForActivityResume() async {
  if (WidgetsBinding.instance.lifecycleState == AppLifecycleState.resumed) {
    return;
  }
  final resumed = Completer<void>();
  final listener = AppLifecycleListener(
    onResume: () {
      if (!resumed.isCompleted) resumed.complete();
    },
  );
  try {
    await resumed.future;
  } finally {
    listener.dispose();
  }
}
