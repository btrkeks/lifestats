import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:logging/logging.dart';
import 'package:opennutritracker/core/data/data_source/lifestats/lifestats_client.dart';
import 'package:opennutritracker/core/data/repository/config_repository.dart';
import 'package:opennutritracker/core/domain/entity/config_entity.dart';
import 'package:opennutritracker/core/domain/usecase/sync_lifestats_weight_usecase.dart';
import 'package:opennutritracker/core/styles/dimens.dart';
import 'package:opennutritracker/core/utils/locator.dart';
import 'package:opennutritracker/features/home/presentation/bloc/home_bloc.dart';
import 'package:opennutritracker/features/profile/presentation/bloc/profile_bloc.dart';
import 'package:opennutritracker/generated/l10n.dart';

/// The user's own server's product name, so it is not localized.
const lifestatsName = 'Lifestats';

/// The user-facing reason for a failed Lifestats request.
String lifestatsErrorText(S s, LifestatsErrorKind? kind, int? statusCode) {
  switch (kind) {
    case LifestatsErrorKind.timeout:
      return s.lifestatsErrorTimeout;
    case LifestatsErrorKind.http:
      return s.lifestatsErrorHttp(statusCode ?? 0);
    case LifestatsErrorKind.badResponse:
      return s.lifestatsErrorBadResponse;
    case LifestatsErrorKind.invalidBaseUrl:
    case LifestatsErrorKind.notConfigured:
      return s.lifestatsBaseUrlInvalid;
    case LifestatsErrorKind.unreachable:
    case null:
      return s.lifestatsErrorUnreachable;
  }
}

/// Settings → Lifestats: the server address, a manual sync, and how the
/// last sync went. The sync itself runs at app start and resume (see
/// [SyncLifestatsWeightUsecase]).
class LifestatsSyncScreen extends StatefulWidget {
  const LifestatsSyncScreen({super.key});

  @override
  State<LifestatsSyncScreen> createState() => _LifestatsSyncScreenState();
}

class _LifestatsSyncScreenState extends State<LifestatsSyncScreen> {
  final _log = Logger('LifestatsSyncScreen');
  final _urlController = TextEditingController();

  ConfigEntity? _config;
  bool _busy = false;
  bool _urlInvalid = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _urlController.dispose();
    super.dispose();
  }

  Future<void> _load({bool resetField = true}) async {
    final config = await locator<ConfigRepository>().getConfig();
    if (!mounted) return;
    setState(() {
      _config = config;
      if (resetField) _urlController.text = config.lifestatsBaseUrl ?? '';
    });
  }

  Future<void> _save() async {
    final text = _urlController.text.trim();
    final parsed = LifestatsClient.parseBaseUrl(text);
    if (text.isNotEmpty && parsed == null) {
      setState(() => _urlInvalid = true);
      return;
    }
    setState(() => _urlInvalid = false);
    FocusScope.of(context).unfocus();
    await locator<ConfigRepository>().setLifestatsBaseUrl(parsed?.toString());
    await _load();
    if (parsed != null) await _syncNow();
  }

  Future<void> _syncNow() async {
    if (!mounted) return;
    setState(() => _busy = true);
    try {
      final written = await locator<SyncLifestatsWeightUsecase>().syncNow();
      if (written > 0) {
        locator<HomeBloc>().add(const LoadItemsEvent());
        locator<ProfileBloc>().add(LoadProfileEvent());
      }
      if (!mounted) return;
      _showMessage(S.of(context).lifestatsSyncedCountLabel(written));
    } on LifestatsException catch (e) {
      _log.info('Lifestats sync failed: $e');
      if (!mounted) return;
      _showMessage(lifestatsErrorText(S.of(context), e.kind, e.statusCode));
    } finally {
      if (mounted) {
        setState(() => _busy = false);
        await _load(resetField: false);
      }
    }
  }

  void _showMessage(String message) {
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }

  String _formatTime(DateTime time) {
    final localeTag = Localizations.localeOf(context).toLanguageTag();
    return DateFormat.yMMMd(localeTag).add_Hm().format(time);
  }

  @override
  Widget build(BuildContext context) {
    final s = S.of(context);
    final theme = Theme.of(context);
    final config = _config;
    return Scaffold(
      appBar: AppBar(title: const Text(lifestatsName)),
      body: config == null
          ? const Center(child: CircularProgressIndicator())
          : ListView(
              padding: const EdgeInsets.all(Dimens.spacing16),
              children: [
                Text(s.lifestatsSyncIntro, style: theme.textTheme.bodyMedium),
                const SizedBox(height: Dimens.spacing16),
                Semantics(
                  identifier: 'lifestats-base-url',
                  child: TextField(
                    controller: _urlController,
                    keyboardType: TextInputType.url,
                    autocorrect: false,
                    decoration: InputDecoration(
                      labelText: s.lifestatsBaseUrlLabel,
                      hintText: 'http://100.64.0.1:3000',
                      errorText: _urlInvalid ? s.lifestatsBaseUrlInvalid : null,
                      border: const OutlineInputBorder(),
                    ),
                    onSubmitted: (_) => _save(),
                  ),
                ),
                const SizedBox(height: Dimens.spacing12),
                Wrap(
                  spacing: Dimens.spacing8,
                  runSpacing: Dimens.spacing8,
                  children: [
                    Semantics(
                      identifier: 'lifestats-save',
                      child: FilledButton(
                        onPressed: _busy ? null : _save,
                        child: Text(s.buttonSaveLabel),
                      ),
                    ),
                    Semantics(
                      identifier: 'lifestats-sync-now',
                      child: OutlinedButton(
                        onPressed: _busy || !config.lifestatsEnabled
                            ? null
                            : _syncNow,
                        child: Text(s.lifestatsSyncNowLabel),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: Dimens.spacing16),
                if (_busy) const LinearProgressIndicator(),
                if (!config.lifestatsEnabled)
                  Text(
                    s.lifestatsDisabledLabel,
                    style: theme.textTheme.bodySmall,
                  )
                else ...[
                  Text(
                    config.lifestatsLastSyncAt != null
                        ? s.lifestatsLastSyncLabel(
                            _formatTime(config.lifestatsLastSyncAt!),
                          )
                        : s.lifestatsNeverSyncedLabel,
                    style: theme.textTheme.bodySmall,
                  ),
                  if (config.lifestatsLastErrorAt != null)
                    Padding(
                      padding: const EdgeInsets.only(top: Dimens.spacing8),
                      child: Text(
                        s.lifestatsLastErrorLabel(
                          _formatTime(config.lifestatsLastErrorAt!),
                          lifestatsErrorText(
                            s,
                            LifestatsErrorKind.fromName(
                              config.lifestatsLastError,
                            ),
                            config.lifestatsLastErrorStatus,
                          ),
                        ),
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.error,
                        ),
                      ),
                    ),
                ],
              ],
            ),
    );
  }
}
