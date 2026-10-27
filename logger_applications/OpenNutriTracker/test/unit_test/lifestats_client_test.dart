import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:opennutritracker/core/data/data_source/lifestats/lifestats_client.dart';

LifestatsClient _client(
  MockClientHandler handler, {
  String? baseUrl = 'http://10.0.2.2:3000',
  Duration timeout = LifestatsClient.defaultTimeout,
}) =>
    LifestatsClient(MockClient(handler), () async => baseUrl, timeout: timeout);

Matcher _throwsKind(LifestatsErrorKind kind, {int? status}) => throwsA(
  isA<LifestatsException>()
      .having((e) => e.kind, 'kind', kind)
      .having((e) => e.statusCode, 'statusCode', status),
);

void main() {
  group('parseBaseUrl', () {
    test('adds http:// when no scheme is given', () {
      expect(
        LifestatsClient.parseBaseUrl('100.64.0.1:3000').toString(),
        'http://100.64.0.1:3000',
      );
    });

    test('trims whitespace and a trailing slash', () {
      expect(
        LifestatsClient.parseBaseUrl(' https://stats.example/api/ ').toString(),
        'https://stats.example/api',
      );
    });

    test('rejects empty, non-http and host-less input', () {
      expect(LifestatsClient.parseBaseUrl(''), isNull);
      expect(LifestatsClient.parseBaseUrl('ftp://host'), isNull);
      expect(LifestatsClient.parseBaseUrl('http://'), isNull);
      expect(LifestatsClient.parseBaseUrl('http://host:3000/?a=1'), isNull);
    });
  });

  test('without a base URL nothing is sent', () async {
    var called = false;
    final client = _client((_) async {
      called = true;
      return http.Response('{}', 200);
    }, baseUrl: '  ');

    expect(await client.isConfigured(), isFalse);
    await expectLater(
      client.getJson('/statistics'),
      _throwsKind(LifestatsErrorKind.notConfigured),
    );
    expect(called, isFalse);
  });

  test('joins the base path and route and sends JSON bodies', () async {
    late http.Request seen;
    final client = _client((request) async {
      seen = request;
      return http.Response('', 204);
    }, baseUrl: 'http://host:3000/lifestats/');

    final result = await client.putJson('/integrations/x/days/2026-10-27', {
      'amount': 3,
    });

    expect(result, isNull);
    expect(seen.method, 'PUT');
    expect(
      seen.url.toString(),
      'http://host:3000/lifestats/integrations/x/days/2026-10-27',
    );
    expect(seen.headers['Content-Type'], startsWith('application/json'));
    expect(jsonDecode(seen.body), {'amount': 3});
  });

  group('getDailyValues', () {
    test('asks for the inclusive range and keeps only logged days', () async {
      late Uri seen;
      final client = _client((request) async {
        seen = request.url;
        return http.Response(
          jsonEncode({
            'statistic_id': 136,
            'days': [
              {'date': '2026-10-25', 'value': null},
              {'date': '2026-10-26', 'value': 82350},
              {'date': '2026-10-27', 'value': 82100},
            ],
          }),
          200,
        );
      });

      final values = await client.getDailyValues(
        136,
        DateTime(2026, 10, 25),
        DateTime(2026, 10, 27),
      );

      expect(seen.path, '/statistics/136/achievements');
      expect(seen.queryParameters, {
        'start': '2026-10-25',
        'end': '2026-10-27',
      });
      expect(values, {
        DateTime(2026, 10, 26): 82350,
        DateTime(2026, 10, 27): 82100,
      });
    });

    test('an unexpected body is a bad response', () async {
      final client = _client((_) async => http.Response('[]', 200));

      await expectLater(
        client.getDailyValues(1, DateTime(2026), DateTime(2026)),
        _throwsKind(LifestatsErrorKind.badResponse),
      );
    });
  });

  test('a non-2xx answer carries its status and message', () async {
    final client = _client(
      (_) async => http.Response('{"error":"Statistic 9 not found"}', 404),
    );

    await expectLater(
      client.getJson('/statistics/9'),
      throwsA(
        isA<LifestatsException>()
            .having((e) => e.kind, 'kind', LifestatsErrorKind.http)
            .having((e) => e.statusCode, 'statusCode', 404)
            .having((e) => e.detail, 'detail', contains('not found')),
      ),
    );
  });

  test('invalid JSON on success is a bad response', () async {
    final client = _client((_) async => http.Response('<html>', 200));

    await expectLater(
      client.getJson('/statistics'),
      _throwsKind(LifestatsErrorKind.badResponse),
    );
  });

  test('a refused connection is unreachable', () async {
    final client = _client(
      (_) async => throw const SocketException('Connection refused'),
    );

    await expectLater(
      client.getJson('/statistics'),
      _throwsKind(LifestatsErrorKind.unreachable),
    );
  });

  test('a client exception is unreachable', () async {
    final client = _client((_) async => throw http.ClientException('reset'));

    await expectLater(
      client.getJson('/statistics'),
      _throwsKind(LifestatsErrorKind.unreachable),
    );
  });

  test('a server that never answers times out', () async {
    final client = _client(
      (_) => Completer<http.Response>().future,
      timeout: const Duration(milliseconds: 10),
    );

    await expectLater(
      client.getJson('/statistics'),
      _throwsKind(LifestatsErrorKind.timeout),
    );
  });
}
