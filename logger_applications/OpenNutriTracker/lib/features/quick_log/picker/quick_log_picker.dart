import 'dart:convert';

import 'package:dynamic_color/dynamic_color.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart';
import 'package:opennutritracker/core/l10n/app_locales.dart';
import 'package:opennutritracker/core/styles/app_palette_resolver.dart';
import 'package:opennutritracker/core/styles/app_theme.dart';
import 'package:opennutritracker/core/styles/dimens.dart';
import 'package:opennutritracker/features/quick_log/picker/picker_request.dart';
import 'package:opennutritracker/generated/l10n.dart';

// Runs in the picker activity's own engine, which is not a Hive owner: this
// library must never reach locator, hive_db_provider or a data source
// (test/unit_test/quick_log/picker_import_guard_test.dart).

/// Registered by `QuickLogPickerActivity` on the picker engine only.
///
///   request() -> String?   the tapped row's PickerRequest JSON
///   submit(Map choice) -> bool   true once the activity is closing
///   dismiss()
const pickerChannel = MethodChannel('com.opennutritracker/quick_log_picker');

/// The body of `quickLogPickerMain` in lib/main.dart.
Future<void> runQuickLogPicker() async {
  WidgetsFlutterBinding.ensureInitialized();
  final raw = await pickerChannel.invokeMethod<String>('request');
  final request = raw == null ? null : PickerRequest.fromJson(jsonDecode(raw));
  if (request == null) {
    await pickerChannel.invokeMethod<void>('dismiss');
    return;
  }
  runApp(
    QuickLogPickerApp(
      request: request,
      onSubmit: (choice) async =>
          await pickerChannel.invokeMethod<bool>('submit', choice.toJson()) ??
          false,
      onDismiss: () => pickerChannel.invokeMethod<void>('dismiss'),
    ),
  );
}

/// The sheet over a scrim, themed like the app from the request's
/// appearance, since this engine cannot read the app's settings.
class QuickLogPickerApp extends StatelessWidget {
  final PickerRequest request;
  final Future<bool> Function(PickerChoice choice) onSubmit;
  final VoidCallback onDismiss;

  const QuickLogPickerApp({
    super.key,
    required this.request,
    required this.onSubmit,
    required this.onDismiss,
  });

  @override
  Widget build(BuildContext context) {
    final appearance = request.appearance;
    final tag = appearance.localeTag.split(RegExp('[-_]'));
    return DynamicColorBuilder(
      builder: (lightDynamic, darkDynamic) {
        final palettes = resolveAppPalettes(
          useMaterialYou: appearance.useMaterialYou,
          accentColor: appearance.accentColor,
          lightDynamicAccent: lightDynamic?.harmonized().primary,
          darkDynamicAccent: darkDynamic?.harmonized().primary,
        );
        return MaterialApp(
          debugShowCheckedModeBanner: false,
          theme: buildAppTheme(palettes.light),
          darkTheme: buildAppTheme(palettes.dark),
          themeMode: ThemeMode.values.byName(appearance.themeMode),
          locale: Locale(tag.first),
          supportedLocales: appLocales(S.supportedLocales),
          localizationsDelegates: const [
            S.delegate,
            GlobalMaterialLocalizations.delegate,
            GlobalCupertinoLocalizations.delegate,
            GlobalWidgetsLocalizations.delegate,
          ],
          color: Colors.transparent,
          home: Scaffold(
            backgroundColor: Colors.transparent,
            body: Stack(
              children: [
                Positioned.fill(
                  child: GestureDetector(
                    onTap: onDismiss,
                    child: const ColoredBox(color: Color(0x66000000)),
                  ),
                ),
                Align(
                  alignment: Alignment.bottomCenter,
                  child: QuickLogPickerSheet(
                    request: request,
                    onSubmit: onSubmit,
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

/// s6 and s7: recent amounts, quantity and unit, a live protein-first
/// preview, and Add.
class QuickLogPickerSheet extends StatefulWidget {
  final PickerRequest request;

  /// Resolves true when the write is done with and the sheet is closing.
  final Future<bool> Function(PickerChoice choice) onSubmit;

  const QuickLogPickerSheet({
    super.key,
    required this.request,
    required this.onSubmit,
  });

  @override
  State<QuickLogPickerSheet> createState() => _QuickLogPickerSheetState();
}

class _QuickLogPickerSheetState extends State<QuickLogPickerSheet> {
  late final TextEditingController _quantity;
  late final NumberFormat _quantityFormat;
  late String _unit;
  bool _submitting = false;

  /// Stands in for a placeholder value so the translated sentence can be
  /// split around it and the value styled on its own.
  static const _slot = '\u0000';

  PickerRequest get _request => widget.request;

  @override
  void initState() {
    super.initState();
    // Grouping off: the field parses what this writes, and "1.000" is a
    // thousand in German.
    _quantityFormat =
        NumberFormat.decimalPattern(
            _request.appearance.localeTag.split(RegExp('[-_]')).first,
          )
          ..maximumFractionDigits = 2
          ..turnOffGrouping();
    _quantity = TextEditingController(
      text: _formatQuantity(_request.initial.quantity),
    )..addListener(() => setState(() {}));
    _unit = _request.initial.unit;
  }

  @override
  void dispose() {
    _quantity.dispose();
    super.dispose();
  }

  String get _localeName => Localizations.localeOf(context).toString();

  String _formatQuantity(double quantity) => _quantityFormat.format(quantity);

  String _unitLabel(String token) =>
      _request.units.firstWhere((unit) => unit.token == token).label;

  bool _isSelected(PickerQuantity amount) {
    final typed = double.tryParse(_quantity.text.trim().replaceAll(',', '.'));
    return amount.unit == _unit && typed == amount.quantity;
  }

  void _select(PickerQuantity amount) {
    _unit = amount.unit;
    _quantity.text = _formatQuantity(amount.quantity);
  }

  Future<void> _add(double baseAmount) async {
    setState(() => _submitting = true);
    final closing = await widget.onSubmit(PickerChoice(baseAmount, _unit));
    if (!closing && mounted) setState(() => _submitting = false);
  }

  @override
  Widget build(BuildContext context) {
    final s = S.of(context);
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final protein = scheme.secondary;
    final muted = TextStyle(color: scheme.onSurfaceVariant);
    final preview = previewFor(_request, _quantity.text, _unit);
    final proteinFormat = NumberFormat.decimalPattern(_localeName)
      ..maximumFractionDigits = 1;
    final integer = NumberFormat.decimalPattern(_localeName);
    final energyUnit = _request.usesKilojoules ? s.kjLabel : s.kcalLabel;
    String energyText(double? energy) => energy == null
        ? s.notAvailableLabel
        : '${integer.format(energy.round())} $energyUnit';

    final per100 = s
        .quickLogPer100Label(_slot, energyText(_per100(_request.energyPerBase)))
        .split(_slot);
    final proteinPer100 = _request.proteinKnown
        ? '${integer.format((_request.proteinPerBase * 100).round())} g'
        : s.notAvailableLabel;
    // The protein phrase runs up to the separator before the energy.
    final afterProtein = per100.length == 2 ? per100.last : '';
    final separator = afterProtein.indexOf('·');
    final proteinTail = separator < 0
        ? ''
        : afterProtein.substring(0, separator).trimRight();

    return Material(
      color: scheme.surface,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(28)),
      ),
      clipBehavior: Clip.antiAlias,
      child: SafeArea(
        top: false,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(
            Dimens.spacing24,
            0,
            Dimens.spacing24,
            Dimens.spacing16,
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Center(
                child: Container(
                  width: 32,
                  height: 4,
                  margin: const EdgeInsets.only(top: 12, bottom: 16),
                  decoration: BoxDecoration(
                    color: scheme.outlineVariant,
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
              ),
              Text(
                _request.title,
                style: theme.textTheme.headlineSmall,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: 2),
              Text.rich(
                TextSpan(
                  style: theme.textTheme.bodyMedium?.merge(muted),
                  children: [
                    TextSpan(text: per100.length == 2 ? per100.first : ''),
                    TextSpan(
                      text: '$proteinPer100$proteinTail',
                      style: TextStyle(
                        color: protein,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    TextSpan(text: afterProtein.substring(proteinTail.length)),
                  ],
                ),
              ),
              if (_request.recent.isNotEmpty) ...[
                const SizedBox(height: Dimens.spacing16),
                Text(
                  s.quickLogRecentAmountsLabel,
                  style: theme.textTheme.labelMedium?.merge(muted),
                ),
                const SizedBox(height: Dimens.spacing8),
                Wrap(
                  spacing: Dimens.spacing8,
                  runSpacing: Dimens.spacing8,
                  children: [
                    for (final (i, amount) in _request.recent.indexed)
                      Semantics(
                        identifier: 'quick-log-picker-chip-$i',
                        child: ChoiceChip(
                          label: Text(
                            '${_formatQuantity(amount.quantity)} ${_unitLabel(amount.unit)}',
                          ),
                          selected: _isSelected(amount),
                          onSelected: (_) => _select(amount),
                        ),
                      ),
                  ],
                ),
              ],
              const SizedBox(height: Dimens.spacing20),
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(
                    child: Semantics(
                      identifier: 'quick-log-picker-quantity',
                      child: TextField(
                        controller: _quantity,
                        keyboardType: const TextInputType.numberWithOptions(
                          decimal: true,
                        ),
                        inputFormatters: [
                          FilteringTextInputFormatter.allow(RegExp(r'[0-9.,]')),
                        ],
                        decoration: InputDecoration(labelText: s.quantityLabel),
                      ),
                    ),
                  ),
                  const SizedBox(width: Dimens.spacing12),
                  SizedBox(
                    width: 128,
                    child: Semantics(
                      identifier: 'quick-log-picker-unit',
                      child: DropdownButtonFormField<String>(
                        initialValue: _unit,
                        isExpanded: true,
                        decoration: InputDecoration(labelText: s.unitLabel),
                        items: [
                          for (final unit in _request.units)
                            DropdownMenuItem(
                              value: unit.token,
                              child: Text(
                                unit.label,
                                overflow: TextOverflow.ellipsis,
                              ),
                            ),
                        ],
                        onChanged: (token) {
                          if (token != null) setState(() => _unit = token);
                        },
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: Dimens.spacing16),
              SizedBox(
                height: 24,
                child: preview == null
                    ? null
                    : _Preview(
                        proteinLine: _request.proteinKnown
                            ? s.quickLogProteinPreview(_slot).split(_slot)
                            : null,
                        proteinValue: proteinFormat.format(preview.protein),
                        energyText: preview.energy == null
                            ? null
                            : energyText(preview.energy),
                        color: protein,
                      ),
              ),
              const SizedBox(height: Dimens.spacing16),
              Semantics(
                identifier: 'quick-log-picker-add',
                child: SizedBox(
                  width: double.infinity,
                  height: 48,
                  child: FilledButton.icon(
                    onPressed: preview == null || _submitting
                        ? null
                        : () => _add(preview.baseAmount),
                    icon: const Icon(Icons.add),
                    label: Text(s.addLabel),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  static double? _per100(double? perBase) =>
      perBase == null ? null : perBase * 100;
}

/// "+30 g protein · 168 kcal": the number leads, protein before energy.
class _Preview extends StatelessWidget {
  /// The translated "+{grams} g protein" split around the value, or null
  /// when the food's protein is unknown.
  final List<String>? proteinLine;
  final String proteinValue;
  final String? energyText;
  final Color color;

  const _Preview({
    required this.proteinLine,
    required this.proteinValue,
    required this.energyText,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final line = proteinLine;
    return Text.rich(
      TextSpan(
        children: [
          if (line != null) ...[
            TextSpan(
              text: '${line.first}$proteinValue',
              style: theme.textTheme.titleLarge?.copyWith(color: color),
            ),
            TextSpan(
              text: line.skip(1).join(),
              style: theme.textTheme.bodyMedium?.copyWith(
                color: color,
                fontWeight: FontWeight.w700,
              ),
            ),
          ],
          if (energyText != null)
            TextSpan(
              text: line == null ? energyText : ' · $energyText',
              style: theme.textTheme.bodyLarge?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
        ],
      ),
      maxLines: 1,
    );
  }
}
