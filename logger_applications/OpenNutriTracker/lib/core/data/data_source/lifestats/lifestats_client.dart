import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:http/http.dart' as http;
import 'package:intl/intl.dart';

/// Why a Lifestats request failed. Stored by name in the config so the
/// settings screen can show the last failure in the user's language.
enum LifestatsErrorKind {
  /// No base URL is set; sync is off.
  notConfigured,

  /// The stored base URL can't be parsed into an http(s) address.
  invalidBaseUrl,

  /// The connection failed: wrong address, server down, or no route to it.
  unreachable,

  /// The server took longer than [LifestatsClient.timeout] to answer.
  timeout,

  /// The server answered with a non-2xx status.
  http,

  /// The server answered 2xx with a body that isn't the expected JSON.
  badResponse;

  static LifestatsErrorKind? fromName(String? name) {
    for (final kind in values) {
      if (kind.name == name) return kind;
    }
    return null;
  }
}

class LifestatsException implements Exception {
  final LifestatsErrorKind kind;

  /// The HTTP status for [LifestatsErrorKind.http], otherwise null.
  final int? statusCode;

  /// Diagnostic detail for logs. Not shown to the user.
  final String detail;

  const LifestatsException(this.kind, {this.statusCode, this.detail = ''});

  @override
  String toString() =>
      'LifestatsException(${kind.name}'
      '${statusCode != null ? ' $statusCode' : ''}'
      '${detail.isNotEmpty ? ': $detail' : ''})';
}

/// HTTP client for a Lifestats web API (the habit and wellness tracker
/// running on the user's own machine). Owns the base URL, transport,
/// timeouts and error mapping, so each feature that reads or writes
/// Lifestats data only names a route.
///
/// Plain `http://` is allowed on purpose, unlike the AI clients'
/// `plaintext_destination_guard`. Lifestats is a server the user runs
/// themselves, typically reached over a Tailscale tailnet (`100.64.0.0/10`),
/// whose WireGuard tunnel already encrypts the traffic.
class LifestatsClient {
  static const defaultTimeout = Duration(seconds: 10);

  final http.Client _httpClient;

  /// Reads the configured base URL on every request, so a change in
  /// Settings applies without rebuilding the client. Null or blank means
  /// sync is off.
  final Future<String?> Function() _baseUrl;
  final Duration timeout;

  LifestatsClient(
    this._httpClient,
    this._baseUrl, {
    this.timeout = defaultTimeout,
  });

  /// Normalises what a user typed into a base URL: trims it, adds `http://`
  /// when no scheme is given and drops a trailing slash. Returns null for
  /// anything that isn't an http(s) URL with a host.
  static Uri? parseBaseUrl(String raw) {
    var text = raw.trim();
    if (text.isEmpty) return null;
    if (!text.contains('://')) text = 'http://$text';
    final uri = Uri.tryParse(text);
    if (uri == null ||
        (uri.scheme != 'http' && uri.scheme != 'https') ||
        uri.host.isEmpty ||
        uri.hasQuery ||
        uri.hasFragment) {
      return null;
    }
    final path = uri.path.endsWith('/')
        ? uri.path.substring(0, uri.path.length - 1)
        : uri.path;
    return uri.replace(path: path);
  }

  Future<bool> isConfigured() async =>
      ((await _baseUrl())?.trim() ?? '').isNotEmpty;

  Future<Object?> getJson(String path, {Map<String, String>? query}) =>
      _send('GET', path, query: query);

  Future<Object?> putJson(String path, Object body) =>
      _send('PUT', path, body: body);

  Future<Object?> postJson(String path, Object body) =>
      _send('POST', path, body: body);

  /// The value logged on each day from [start] to [end] (inclusive) for the
  /// statistic [statisticId], keyed by local midnight. Days without a
  /// value are absent. Numeric statistics only (IntegerAmount, Summing,
  /// and seconds for time statistics).
  Future<Map<DateTime, num>> getDailyValues(
    int statisticId,
    DateTime start,
    DateTime end,
  ) async {
    final dateFormat = DateFormat('yyyy-MM-dd');
    final json = await getJson(
      '/statistics/$statisticId/achievements',
      query: {'start': dateFormat.format(start), 'end': dateFormat.format(end)},
    );
    final days = json is Map<String, dynamic> ? json['days'] : null;
    if (days is! List) {
      throw const LifestatsException(
        LifestatsErrorKind.badResponse,
        detail: 'achievements without a days list',
      );
    }
    final values = <DateTime, num>{};
    for (final day in days) {
      if (day is! Map<String, dynamic>) continue;
      final value = day['value'];
      final date = DateTime.tryParse(day['date'] as String? ?? '');
      if (value is! num || date == null) continue;
      values[DateTime(date.year, date.month, date.day)] = value;
    }
    return values;
  }

  Future<Object?> _send(
    String method,
    String path, {
    Map<String, String>? query,
    Object? body,
  }) async {
    final rawBase = (await _baseUrl())?.trim() ?? '';
    if (rawBase.isEmpty) {
      throw const LifestatsException(LifestatsErrorKind.notConfigured);
    }
    final base = parseBaseUrl(rawBase);
    if (base == null) {
      throw LifestatsException(
        LifestatsErrorKind.invalidBaseUrl,
        detail: rawBase,
      );
    }
    final uri = base.replace(path: '${base.path}$path', queryParameters: query);
    final request = http.Request(method, uri)
      ..headers['Accept'] = 'application/json';
    if (body != null) {
      request.headers['Content-Type'] = 'application/json';
      request.body = jsonEncode(body);
    }

    final http.Response response;
    try {
      response = await http.Response.fromStream(
        await _httpClient.send(request).timeout(timeout),
      ).timeout(timeout);
    } on TimeoutException {
      throw LifestatsException(
        LifestatsErrorKind.timeout,
        detail: '$method $uri',
      );
    } on SocketException catch (e) {
      throw LifestatsException(
        LifestatsErrorKind.unreachable,
        detail: '$method $uri: ${e.message}',
      );
    } on http.ClientException catch (e) {
      throw LifestatsException(
        LifestatsErrorKind.unreachable,
        detail: '$method $uri: ${e.message}',
      );
    }

    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw LifestatsException(
        LifestatsErrorKind.http,
        statusCode: response.statusCode,
        detail: '$method $uri: ${_errorMessage(response.body)}',
      );
    }
    if (response.body.isEmpty) return null;
    try {
      return jsonDecode(response.body);
    } on FormatException catch (e) {
      throw LifestatsException(
        LifestatsErrorKind.badResponse,
        detail: '$method $uri: ${e.message}',
      );
    }
  }

  /// Lifestats answers errors as `{"error": "message"}` or, on newer
  /// routes, `{"error": {"code": ..., "message": ...}}`.
  static String _errorMessage(String body) {
    try {
      final error = (jsonDecode(body) as Map<String, dynamic>)['error'];
      if (error is String) return error;
      if (error is Map<String, dynamic>) {
        return '${error['code'] ?? ''} ${error['message'] ?? ''}'.trim();
      }
    } catch (_) {
      // Not JSON; fall through to the raw body.
    }
    return body.length > 200 ? body.substring(0, 200) : body;
  }
}
