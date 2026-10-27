import 'package:flutter/material.dart';
import 'package:opennutritracker/core/domain/entity/intake_entity.dart';
import 'package:opennutritracker/core/domain/usecase/get_config_usecase.dart';
import 'package:opennutritracker/core/utils/calc/intake_time_calc.dart';
import 'package:opennutritracker/core/utils/calc/unit_calc.dart';
import 'package:opennutritracker/core/utils/custom_text_input_formatter.dart';
import 'package:opennutritracker/core/utils/locator.dart';
import 'package:opennutritracker/generated/l10n.dart';

/// What the user confirmed in [EditDialog].
class EditIntakeResult {
  /// The amount in metric units.
  final double amount;

  /// The newly picked time of day, or null when the time was left alone.
  final TimeOfDay? time;

  const EditIntakeResult({required this.amount, this.time});
}

/// The `updateIntakeItem` fields for [result]: the amount, and the new
/// timestamp when a time was picked, kept on [intake]'s logical day.
Future<Map<String, dynamic>> editedIntakeFields(
  IntakeEntity intake,
  EditIntakeResult result,
) async {
  final time = result.time;
  if (time == null) return {'amount': result.amount};
  final config = await locator<GetConfigUsecase>().getConfig();
  return {
    'amount': result.amount,
    'dateTime': IntakeTimeCalc.withTimeOfDay(
      intake.dateTime,
      time.hour,
      time.minute,
      config.dayStartOffsetTotalMinutes,
    ),
  };
}

class EditDialog extends StatefulWidget {
  final IntakeEntity intakeEntity;
  final bool usesImperialUnits;

  const EditDialog({
    super.key,
    required this.intakeEntity,
    required this.usesImperialUnits,
  });

  @override
  State<StatefulWidget> createState() => _EditDialogState();
}

class _EditDialogState extends State<EditDialog> {
  late TextEditingController amountEditingController;

  /// The stored time, or null for an intake that only knows its day.
  late final TimeOfDay? _storedTime;
  TimeOfDay? _pickedTime;

  @override
  void initState() {
    super.initState();
    double initialAmount = _convertValue(
      widget.intakeEntity.amount,
      widget.intakeEntity.meal.mealUnit,
    );
    amountEditingController = TextEditingController(
      text: initialAmount.toStringAsFixed(2),
    );
    final dateTime = widget.intakeEntity.dateTime;
    _storedTime = IntakeTimeCalc.hasClockTime(dateTime)
        ? TimeOfDay.fromDateTime(dateTime.toLocal())
        : null;
  }

  Future<void> _pickTime() async {
    final picked = await showTimePicker(
      context: context,
      initialTime: _pickedTime ?? _storedTime ?? TimeOfDay.now(),
    );
    if (picked != null) setState(() => _pickedTime = picked);
  }

  @override
  Widget build(BuildContext context) {
    final shownTime = _pickedTime ?? _storedTime;
    return AlertDialog(
      title: Text(S.of(context).editItemDialogTitle),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          TextFormField(
            controller: amountEditingController,
            keyboardType: const TextInputType.numberWithOptions(decimal: true),
            inputFormatters: CustomTextInputFormatter.doubleOnly(),
            decoration: InputDecoration(
              labelText: S.of(context).quantityLabel,
              suffixText: _convertUnit(widget.intakeEntity.meal.mealUnit ?? ''),
            ),
          ),
          const SizedBox(height: 16),
          Semantics(
            identifier: 'edit-intake-time',
            button: true,
            child: InkWell(
              onTap: _pickTime,
              child: InputDecorator(
                decoration: InputDecoration(
                  labelText: S.of(context).intakeTimeLabel,
                  suffixIcon: const Icon(Icons.schedule),
                ),
                child: Text(
                  shownTime?.format(context) ??
                      S.of(context).intakeTimeNotSetLabel,
                ),
              ),
            ),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () {
            final newAmount = double.tryParse(
              amountEditingController.text.replaceAll(',', '.'),
            );
            if (newAmount == null) return;
            Navigator.of(context).pop(
              EditIntakeResult(
                amount: _convertBackToMetricValue(
                  newAmount,
                  widget.intakeEntity.meal.mealUnit,
                ),
                time: _pickedTime != _storedTime ? _pickedTime : null,
              ),
            );
          },
          child: Text(S.of(context).dialogOKLabel),
        ),
        TextButton(
          onPressed: () {
            Navigator.of(context).pop();
          },
          child: Text(S.of(context).dialogCancelLabel),
        ),
      ],
    );
  }

  double _convertValue(double value, String? unit) {
    switch (unit) {
      case 'g':
        return widget.usesImperialUnits ? UnitCalc.gToOz(value) : value;
      case 'ml':
        return widget.usesImperialUnits ? UnitCalc.mlToFlOz(value) : value;
      default:
        return value;
    }
  }

  double _convertBackToMetricValue(double value, String? unit) {
    switch (unit) {
      case 'g':
        return widget.usesImperialUnits ? UnitCalc.ozToG(value) : value;
      case 'ml':
        return widget.usesImperialUnits ? UnitCalc.flOzToMl(value) : value;
      default:
        return value;
    }
  }

  String _convertUnit(String unit) {
    switch (unit) {
      case 'g':
        return widget.usesImperialUnits ? 'oz' : 'g';
      case 'ml':
        return widget.usesImperialUnits ? 'fl.oz' : 'ml';
      default:
        return unit;
    }
  }
}
