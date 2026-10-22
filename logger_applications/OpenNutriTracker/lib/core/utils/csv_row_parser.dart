/// Minimal CSV row splitter shared between the meal-import and recipe-
/// import flows. The food-label use case doesn't justify pulling in a
/// full CSV package: rows are short, headers are known, and the only
/// quoting subtlety worth handling here is `""` as the escape for an
/// embedded quote.
///
/// Decimal-comma payloads (a European user writing `1,5` for "one
/// point five") have to be wrapped in double quotes (`"1,5"`) before
/// they reach this splitter, because there is no ambiguity-free way
/// to tell `288,12,5` (288 followed by 12.5) apart from `288,12,5`
/// (three integers) without column-aware reasoning that the parser
/// doesn't have. The importers surface a hint about quoting in their
/// "too many columns" error path, and the in-app sample CSVs set the
/// expectation by example.
class CsvRowParser {
  /// Split [line] into cells. Handles double-quoted fields (with
  /// `""` as the escape for an embedded quote). All other commas
  /// outside of quotes are treated as field separators.
  static List<String> splitRow(String line) {
    final cells = <String>[];
    final buf = StringBuffer();
    var inQuotes = false;

    for (var i = 0; i < line.length; i++) {
      final ch = line[i];
      if (ch == '"') {
        if (inQuotes && i + 1 < line.length && line[i + 1] == '"') {
          buf.write('"');
          i++;
        } else {
          inQuotes = !inQuotes;
        }
      } else if (ch == ',' && !inQuotes) {
        cells.add(buf.toString());
        buf.clear();
      } else {
        buf.write(ch);
      }
    }
    cells.add(buf.toString());
    return cells;
  }

  /// Parse [raw] as a double, accepting either `.` or `,` as the
  /// decimal mark. The comma-as-decimal acceptance applies to
  /// individual cell values *after* splitting, so a quoted
  /// decimal-comma cell (`"1,5"`) lands here as the string `1,5` and
  /// resolves to `1.5`. Returns null on empty or unparseable input.
  static double? parseDoubleOrNull(String? raw) {
    if (raw == null) return null;
    final trimmed = raw.trim();
    if (trimmed.isEmpty) return null;
    return double.tryParse(trimmed.replaceAll(',', '.'));
  }
}
