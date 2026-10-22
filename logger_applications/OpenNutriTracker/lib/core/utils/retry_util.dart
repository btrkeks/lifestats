/// Retries [fn] up to [attempts] times with exponential backoff (1 s, 2 s, 4 s…).
/// If [shouldRetry] is provided, only retries when it returns true for the thrown error.
Future<T> withRetry<T>(
  Future<T> Function() fn, {
  int attempts = 3,
  bool Function(Object)? shouldRetry,
}) async {
  for (var i = 0; i < attempts; i++) {
    try {
      return await fn();
    } catch (e) {
      final isLast = i == attempts - 1;
      if (isLast || (shouldRetry != null && !shouldRetry(e))) rethrow;
      await Future.delayed(Duration(seconds: 1 << i));
    }
  }
  throw StateError('unreachable');
}
