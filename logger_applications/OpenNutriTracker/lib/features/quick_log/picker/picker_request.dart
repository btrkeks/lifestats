class PickerUnit {
  final String token;
  final String label;
  final double basePerUnit;
  const PickerUnit(this.token, this.label, this.basePerUnit);

  Map<String, Object?> toJson() => {
    'token': token,
    'label': label,
    'basePerUnit': basePerUnit,
  };
}

class PickerQuantity {
  final double quantity;
  final String unit;
  const PickerQuantity(this.quantity, this.unit);

  Map<String, Object?> toJson() => {'quantity': quantity, 'unit': unit};
}

class PickerAppearance {
  final String localeTag;
  final String themeMode;
  final bool useMaterialYou;
  final int? accentColor;
  const PickerAppearance({
    required this.localeTag,
    required this.themeMode,
    required this.useMaterialYou,
    required this.accentColor,
  });

  Map<String, Object?> toJson() => {
    'localeTag': localeTag,
    'themeMode': themeMode,
    'useMaterialYou': useMaterialYou,
    'accentColor': accentColor,
  };
}

class PickerRequest {
  final String foodKey;
  final String title;
  final double proteinPerBase;
  final bool proteinKnown;
  final double? energyPerBase;
  final bool usesKilojoules;
  final List<PickerUnit> units;
  final List<PickerQuantity> recent;
  final PickerQuantity initial;
  final PickerAppearance appearance;

  const PickerRequest({
    required this.foodKey,
    required this.title,
    required this.proteinPerBase,
    required this.proteinKnown,
    required this.energyPerBase,
    required this.usesKilojoules,
    required this.units,
    required this.recent,
    required this.initial,
    required this.appearance,
  });

  Map<String, Object?> toJson() => {
    'foodKey': foodKey,
    'title': title,
    'proteinPerBase': proteinPerBase,
    'proteinKnown': proteinKnown,
    'energyPerBase': energyPerBase,
    'usesKilojoules': usesKilojoules,
    'units': units.map((u) => u.toJson()).toList(),
    'recent': recent.map((q) => q.toJson()).toList(),
    'initial': initial.toJson(),
    'appearance': appearance.toJson(),
  };

  static PickerRequest? fromJson(Object? json) {
    if (json is! Map<String, Object?>) return null;
    final foodKey = json['foodKey'];
    final title = json['title'];
    final protein = _nonnegative(json['proteinPerBase']);
    final proteinKnown = json['proteinKnown'];
    final energy = _nonnegative(json['energyPerBase']);
    final usesKilojoules = json['usesKilojoules'];
    final rawUnits = json['units'];
    final rawRecent = json['recent'];
    final rawAppearance = json['appearance'];
    if (foodKey is! String ||
        foodKey.trim().isEmpty ||
        title is! String ||
        title.trim().isEmpty ||
        protein == null ||
        proteinKnown is! bool ||
        !json.containsKey('energyPerBase') ||
        (json['energyPerBase'] != null && energy == null) ||
        usesKilojoules is! bool ||
        rawUnits is! List ||
        rawUnits.isEmpty ||
        rawRecent is! List ||
        rawRecent.length > 4 ||
        rawAppearance is! Map<String, Object?>) {
      return null;
    }
    final units = <PickerUnit>[];
    final tokens = <String>{};
    for (final raw in rawUnits) {
      if (raw is! Map<String, Object?>) return null;
      final token = raw['token'];
      final label = raw['label'];
      final factor = _nonnegative(raw['basePerUnit']);
      if (token is! String ||
          !const ['g', 'ml', 'serving', 'oz', 'fl.oz'].contains(token) ||
          !tokens.add(token) ||
          label is! String ||
          label.trim().isEmpty ||
          factor == null ||
          factor == 0) {
        return null;
      }
      units.add(PickerUnit(token, label, factor));
    }
    final initial = _quantity(json['initial'], units);
    if (initial == null) return null;
    final recent = <PickerQuantity>[];
    for (final raw in rawRecent) {
      final quantity = _quantity(raw, units);
      if (quantity == null) return null;
      recent.add(quantity);
    }
    final locale = rawAppearance['localeTag'];
    final theme = rawAppearance['themeMode'];
    final materialYou = rawAppearance['useMaterialYou'];
    final accent = rawAppearance['accentColor'];
    if (!rawAppearance.containsKey('accentColor') ||
        locale is! String ||
        !RegExp(r'^[a-zA-Z]{2,3}([_-][a-zA-Z0-9]{2,8})*$').hasMatch(locale) ||
        theme is! String ||
        !const ['system', 'light', 'dark'].contains(theme) ||
        materialYou is! bool ||
        (accent != null &&
            (accent is! int || accent < 0 || accent > 0xffffffff))) {
      return null;
    }
    return PickerRequest(
      foodKey: foodKey,
      title: title,
      proteinPerBase: protein,
      proteinKnown: proteinKnown,
      energyPerBase: energy,
      usesKilojoules: usesKilojoules,
      units: units,
      recent: recent,
      initial: initial,
      appearance: PickerAppearance(
        localeTag: locale,
        themeMode: theme,
        useMaterialYou: materialYou,
        accentColor: accent as int?,
      ),
    );
  }
}

double? _nonnegative(Object? value) =>
    value is num && value.isFinite && value >= 0 ? value.toDouble() : null;

PickerQuantity? _quantity(Object? raw, List<PickerUnit> units) {
  if (raw is! Map<String, Object?>) return null;
  final quantity = _nonnegative(raw['quantity']);
  final token = raw['unit'];
  if (quantity == null || quantity == 0 || token is! String) return null;
  final unit = units.where((unit) => unit.token == token).firstOrNull;
  if (unit == null || !_validAmount(quantity * unit.basePerUnit)) return null;
  return PickerQuantity(quantity, token);
}

class PickerChoice {
  final double baseAmount;
  final String unit;
  const PickerChoice(this.baseAmount, this.unit);

  Map<String, Object?> toJson() => {'baseAmount': baseAmount, 'unit': unit};
}

/// Display rounding matches the protein-first line and whole-energy rows.
({double baseAmount, double protein, double? energy})? previewFor(
  PickerRequest request,
  String quantityText,
  String unit,
) {
  final quantity = double.tryParse(quantityText.trim().replaceAll(',', '.'));
  final selected = request.units.where((u) => u.token == unit).firstOrNull;
  if (quantity == null || selected == null) return null;
  final base = quantity * selected.basePerUnit;
  if (!_validAmount(base)) return null;
  final protein = base * request.proteinPerBase;
  final energy = request.energyPerBase == null
      ? null
      : base * request.energyPerBase!;
  if (!protein.isFinite || (energy != null && !energy.isFinite)) return null;
  return (
    baseAmount: base,
    protein: double.parse(protein.toStringAsFixed(1)),
    energy: energy?.roundToDouble(),
  );
}

bool _validAmount(double amount) =>
    amount.isFinite && amount > 0 && amount <= 10000;
