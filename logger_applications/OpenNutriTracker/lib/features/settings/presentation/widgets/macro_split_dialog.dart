import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:opennutritracker/core/utils/calc/macro_calc.dart';
import 'package:opennutritracker/core/utils/calc/protein_goal_calc.dart';
import 'package:opennutritracker/features/home/presentation/bloc/home_bloc.dart';
import 'package:opennutritracker/features/settings/presentation/bloc/settings_bloc.dart';
import 'package:opennutritracker/generated/l10n.dart';

/// Parses a g/kg factor typed with either decimal separator. Returns null
/// for anything that isn't a number inside the allowed range.
@visibleForTesting
double? parseProteinGramsPerKg(String text) {
  final parsed = double.tryParse(text.trim().replaceAll(',', '.'));
  if (parsed == null ||
      parsed < ProteinGoalCalc.minGramsPerKg ||
      parsed > ProteinGoalCalc.maxGramsPerKg) {
    return null;
  }
  return (parsed * 10).round() / 10;
}

/// Sets the two inputs of the macro goals: protein in grams per kg of body
/// weight, and how carbs and fat split the calories protein leaves. Fat is
/// whatever carbs don't take, so only carbs has a control of its own.
class MacroSplitDialog extends StatefulWidget {
  final SettingsBloc settingsBloc;
  final HomeBloc homeBloc;

  const MacroSplitDialog({
    super.key,
    required this.settingsBloc,
    required this.homeBloc,
  });

  @override
  State<MacroSplitDialog> createState() => _MacroSplitDialogState();
}

class _MacroSplitDialogState extends State<MacroSplitDialog> {
  static final int _defaultCarbsSharePct =
      (MacroCalc.carbsShareOfRemainder() * 100).round();
  static const int _minCarbsSharePct = 5;
  static const int _maxCarbsSharePct = 95;

  double _proteinGramsPerKg = ProteinGoalCalc.defaultGramsPerKg;
  int _carbsSharePct = _defaultCarbsSharePct;
  ProteinReferenceWeight? _referenceWeight;
  bool _loaded = false;

  /// The field the user typed in most recently. On save only this field's
  /// pending (unsubmitted) text is applied — last write wins, as before the
  /// protein rework.
  _MacroField? _lastEdited;

  late final TextEditingController _proteinController;
  late final TextEditingController _carbsController;

  @override
  void initState() {
    super.initState();
    _proteinController = TextEditingController();
    _carbsController = TextEditingController();
    _load();
  }

  @override
  void dispose() {
    _proteinController.dispose();
    _carbsController.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    final carbs = await widget.settingsBloc.getUserCarbGoalPct();
    final fat = await widget.settingsBloc.getUserFatGoalPct();
    final gramsPerKg = await widget.settingsBloc.getUserProteinGramsPerKg();
    final referenceWeight = await widget.settingsBloc
        .getProteinReferenceWeight();
    if (!mounted) return;
    setState(() {
      _carbsSharePct =
          (MacroCalc.carbsShareOfRemainder(
                    userCarbsGoal: carbs,
                    userFatsGoal: fat,
                  ) *
                  100)
              .round()
              .clamp(_minCarbsSharePct, _maxCarbsSharePct);
      _proteinGramsPerKg = gramsPerKg ?? ProteinGoalCalc.defaultGramsPerKg;
      _referenceWeight = referenceWeight;
      _loaded = true;
    });
    _syncControllers();
  }

  void _syncControllers() {
    _proteinController.text = _proteinGramsPerKg.toStringAsFixed(1);
    _carbsController.text = _carbsSharePct.toString();
  }

  void _applyProteinInput() {
    final parsed = parseProteinGramsPerKg(_proteinController.text);
    if (parsed != null) setState(() => _proteinGramsPerKg = parsed);
    _syncControllers();
  }

  void _applyCarbsInput() {
    final parsed = int.tryParse(_carbsController.text);
    if (parsed != null &&
        parsed >= _minCarbsSharePct &&
        parsed <= _maxCarbsSharePct) {
      setState(() => _carbsSharePct = parsed);
    }
    _syncControllers();
  }

  Future<void> _save() async {
    switch (_lastEdited) {
      case _MacroField.protein:
        _applyProteinInput();
      case _MacroField.carbs:
        _applyCarbsInput();
      case null:
        break;
    }
    await widget.settingsBloc.setMacroGoals(
      carbsSharePct: _carbsSharePct,
      proteinGramsPerKg: _proteinGramsPerKg,
    );
    widget.settingsBloc.add(LoadSettingsEvent());
    widget.homeBloc.add(const LoadItemsEvent());
    await widget.settingsBloc.updateTrackedDay(DateTime.now());
    if (!mounted) return;
    Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    final s = S.of(context);
    final textTheme = Theme.of(context).textTheme;
    final referenceWeight = _referenceWeight;
    return AlertDialog(
      title: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Expanded(
            child: Text(
              s.settingsMacroSplitLabel,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
            ),
          ),
          const SizedBox(width: 8),
          TextButton(
            onPressed: _loaded
                ? () {
                    setState(() {
                      _proteinGramsPerKg = ProteinGoalCalc.defaultGramsPerKg;
                      _carbsSharePct = _defaultCarbsSharePct;
                    });
                    _syncControllers();
                  }
                : null,
            child: Text(s.buttonResetLabel),
          ),
        ],
      ),
      content: !_loaded
          ? const SizedBox(
              height: 80,
              child: Center(child: CircularProgressIndicator()),
            )
          : SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  _ValueField(
                    label: s.settingsMacroSplitProteinLabel,
                    controller: _proteinController,
                    suffix: 'g/kg',
                    decimal: true,
                    onChanged: (_) => _lastEdited = _MacroField.protein,
                    onSubmitted: _applyProteinInput,
                  ),
                  _ColoredSlider(
                    color: Colors.blue,
                    semanticIdentifier: 'macro-split-protein',
                    min: ProteinGoalCalc.minGramsPerKg,
                    max: ProteinGoalCalc.maxGramsPerKg,
                    divisions:
                        ((ProteinGoalCalc.maxGramsPerKg -
                                    ProteinGoalCalc.minGramsPerKg) *
                                10)
                            .round(),
                    value: _proteinGramsPerKg,
                    onChanged: (v) => setState(
                      () => _proteinGramsPerKg = (v * 10).round() / 10,
                    ),
                    onChangeEnd: _syncControllers,
                  ),
                  if (referenceWeight != null)
                    Text(
                      s.settingsMacroSplitProteinPreview(
                        ProteinGoalCalc.goalGrams(
                          referenceWeight.weightKg,
                          gramsPerKg: _proteinGramsPerKg,
                        ).toStringAsFixed(0),
                        referenceWeight.weightKg.toStringAsFixed(1),
                      ),
                      style: textTheme.bodySmall,
                    ),
                  const SizedBox(height: 16),
                  Text(
                    s.settingsMacroSplitRemainderLabel,
                    style: textTheme.titleSmall,
                  ),
                  const SizedBox(height: 4),
                  _ValueField(
                    label: s.carbsLabel,
                    controller: _carbsController,
                    suffix: '%',
                    decimal: false,
                    onChanged: (_) => _lastEdited = _MacroField.carbs,
                    onSubmitted: _applyCarbsInput,
                  ),
                  _ColoredSlider(
                    color: Colors.orange,
                    semanticIdentifier: 'macro-split-carbs',
                    min: _minCarbsSharePct.toDouble(),
                    max: _maxCarbsSharePct.toDouble(),
                    divisions: _maxCarbsSharePct - _minCarbsSharePct,
                    value: _carbsSharePct.toDouble(),
                    onChanged: (v) =>
                        setState(() => _carbsSharePct = v.round()),
                    onChangeEnd: _syncControllers,
                  ),
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          s.fatLabel,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      const SizedBox(width: 8),
                      Text('${100 - _carbsSharePct} %'),
                    ],
                  ),
                ],
              ),
            ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(s.dialogCancelLabel),
        ),
        Semantics(
          identifier: 'macro-split-save',
          child: TextButton(
            onPressed: _loaded ? _save : null,
            child: Text(s.dialogOKLabel),
          ),
        ),
      ],
    );
  }
}

class _ValueField extends StatelessWidget {
  final String label;
  final TextEditingController controller;
  final String suffix;
  final bool decimal;
  final ValueChanged<String> onChanged;
  final VoidCallback onSubmitted;

  const _ValueField({
    required this.label,
    required this.controller,
    required this.suffix,
    required this.decimal,
    required this.onChanged,
    required this.onSubmitted,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Expanded(child: Text(label)),
        SizedBox(
          // Scale the field width with the user's text setting so the
          // value and its unit suffix stay readable at large font scales.
          width: MediaQuery.textScalerOf(context).scale(112),
          child: TextField(
            controller: controller,
            keyboardType: TextInputType.numberWithOptions(decimal: decimal),
            inputFormatters: [
              decimal
                  ? FilteringTextInputFormatter.allow(RegExp(r'[0-9.,]'))
                  : FilteringTextInputFormatter.digitsOnly,
            ],
            textAlign: TextAlign.right,
            decoration: InputDecoration(
              suffixText: suffix,
              isDense: true,
              contentPadding: const EdgeInsets.symmetric(
                horizontal: 8,
                vertical: 12,
              ),
            ),
            onChanged: onChanged,
            onSubmitted: (_) => onSubmitted(),
            onEditingComplete: onSubmitted,
          ),
        ),
      ],
    );
  }
}

class _ColoredSlider extends StatelessWidget {
  final Color color;
  final String semanticIdentifier;
  final double min;
  final double max;
  final int divisions;
  final double value;
  final ValueChanged<double> onChanged;
  final VoidCallback onChangeEnd;

  const _ColoredSlider({
    required this.color,
    required this.semanticIdentifier,
    required this.min,
    required this.max,
    required this.divisions,
    required this.value,
    required this.onChanged,
    required this.onChangeEnd,
  });

  @override
  Widget build(BuildContext context) {
    return SliderTheme(
      data: SliderThemeData(
        activeTrackColor: color,
        thumbColor: color,
        inactiveTrackColor: color.withValues(alpha: 0.2),
      ),
      child: Semantics(
        identifier: semanticIdentifier,
        child: Slider(
          min: min,
          max: max,
          value: value.clamp(min, max),
          divisions: divisions,
          onChanged: onChanged,
          onChangeEnd: (_) => onChangeEnd(),
        ),
      ),
    );
  }
}

enum _MacroField { protein, carbs }
