import { CellInput } from '@/components/presentation/foundation/editors/cell-input';
import Icon from '@/components/presentation/foundation/icon';
import IconButton from '@/components/presentation/foundation/icon-button';
import WeightFormat from '@/components/presentation/foundation/weight-format';
import {
  resolveBlur,
  resolveCheck,
  RowDraft,
  SetRow,
  WeightColumn,
  weightCellText,
} from '@/components/presentation/workout/weighted/set-rows';
import { rounding, spacing, useAppTheme } from '@/hooks/useAppTheme';
import { formatRepsTarget } from '@/models/blueprint-models';
import { WeightedChange } from '@/models/session-models';
import { shortFormatWeightUnit } from '@/models/weight';
import { useTranslate } from '@tolgee/react';
import { useRef, useState } from 'react';
import { Keyboard, Platform, TextInput, View } from 'react-native';
import { Text } from 'react-native-paper';
import { match } from 'ts-pattern';

const columns = {
  set: { width: spacing[8] },
  previous: { flex: 1 },
  weight: { width: spacing[16] },
  reps: { width: spacing[14] },
  check: { width: spacing[11], alignItems: 'center' },
} as const;

const rowStyle = {
  flexDirection: 'row',
  alignItems: 'center',
  gap: spacing[1],
  paddingHorizontal: spacing[1],
  borderRadius: rounding.roundedRectangleRadius,
} as const;

export function SetTableHeader(props: { column: WeightColumn }) {
  const { t } = useTranslate();
  const { colors } = useAppTheme();
  const unit = shortFormatWeightUnit(props.column.unit);
  const labelStyle = { color: colors.onSurfaceVariant, textAlign: 'center' } as const;
  return (
    <View style={rowStyle}>
      <Text variant="labelMedium" style={[columns.set, labelStyle]}>
        {t('exercise.set.label')}
      </Text>
      <Text variant="labelMedium" style={[columns.previous, labelStyle, { textAlign: 'left' }]}>
        {t('exercise.previous.label')}
      </Text>
      {props.column.resistance !== 'none' && (
        <Text variant="labelMedium" style={[columns.weight, labelStyle]}>
          {props.column.resistance === 'bodyweight' ? `+${unit}` : unit}
        </Text>
      )}
      <Text variant="labelMedium" style={[columns.reps, labelStyle]}>
        {t('exercise.reps.label')}
      </Text>
      <View style={columns.check}>
        <Icon source="check" size={18} color={colors.onSurfaceVariant} />
      </View>
    </View>
  );
}

export function SetTableRow(props: {
  row: SetRow;
  column: WeightColumn;
  isReadonly: boolean;
  isNext: boolean;
  onChange: (change: WeightedChange) => void;
}) {
  const { row, column, onChange } = props;
  const { colors } = useAppTheme();
  const [draft, setDraft] = useState<RowDraft>({});
  const weightRef = useRef<TextInput>(null);
  const repsRef = useRef<TextInput>(null);
  const isChecked = row.status.kind === 'checked';
  const textColor = isChecked ? colors.onPrimaryContainer : colors.onSurface;

  const onCheckPress = () =>
    match(resolveCheck(row, draft, column))
      .with({ kind: 'commit' }, ({ change }) => {
        onChange(change);
        setDraft({});
        Keyboard.dismiss();
      })
      .with({ kind: 'uncheck' }, ({ change, keepRepsText }) => {
        onChange(change);
        setDraft((d) => ({ ...d, repsText: keepRepsText }));
      })
      .with({ kind: 'focus', field: 'weight' }, () => weightRef.current?.focus())
      .with({ kind: 'focus', field: 'reps' }, () => repsRef.current?.focus())
      .exhaustive();

  const onBlur = (field: 'weight' | 'reps') => {
    const change = resolveBlur(row, field, draft, column);
    if (change) {
      onChange(change);
    }
    // Reps typed into an unchecked row wait for ✓; everything else reverts to the model.
    if (field === 'weight') {
      setDraft((d) => ({ ...d, weightText: undefined }));
    } else if (isChecked) {
      setDraft((d) => ({ ...d, repsText: undefined }));
    }
  };

  const weightText = draft.weightText ?? weightCellText(row.weight, column.unit);
  const repsText = draft.repsText ?? (row.status.kind === 'checked' ? String(row.status.reps) : '');
  const repsPlaceholder =
    row.status.kind === 'unchecked'
      ? (row.status.suggestedReps?.toString() ?? (row.target.max > 0 ? formatRepsTarget(row.target) : '-'))
      : undefined;
  const cellStyle = isChecked ? { backgroundColor: 'transparent', color: textColor, fontWeight: 'bold' as const } : {};

  return (
    <View
      testID="set-row"
      style={[rowStyle, { minHeight: spacing[11] }, isChecked && { backgroundColor: colors.primaryContainer }]}
    >
      <Text
        variant="bodyLarge"
        style={[
          columns.set,
          { textAlign: 'center', color: props.isNext ? colors.primary : textColor },
          props.isNext && { fontWeight: 'bold' },
        ]}
      >
        {row.index + 1}
      </Text>
      <Text
        variant="bodyMedium"
        numberOfLines={1}
        style={[columns.previous, { color: isChecked ? colors.onPrimaryContainer : colors.onSurfaceVariant }]}
      >
        {row.previous ? (
          <>
            {column.resistance !== 'none' && (
              <>
                <WeightFormat
                  weight={row.previous.weight}
                  usesBodyweight={column.resistance === 'bodyweight'}
                  color={isChecked ? 'onPrimaryContainer' : 'onSurfaceVariant'}
                  fontSize="text-sm"
                />
                {' × '}
              </>
            )}
            {row.previous.reps}
          </>
        ) : (
          '-'
        )}
      </Text>
      {column.resistance !== 'none' && (
        <CellInput
          ref={weightRef}
          testID="set-row-weight"
          style={[columns.weight, cellStyle]}
          value={weightText}
          placeholder="0"
          // iOS's decimal pad has no minus key, and assisted bodyweight sets are negative.
          keyboardType={
            column.resistance === 'bodyweight' && Platform.OS === 'ios' ? 'numbers-and-punctuation' : 'decimal-pad'
          }
          editable={!props.isReadonly}
          onChangeText={(text) => setDraft((d) => ({ ...d, weightText: text }))}
          onBlur={() => onBlur('weight')}
        />
      )}
      <CellInput
        ref={repsRef}
        testID="set-row-reps"
        style={[columns.reps, cellStyle]}
        value={repsText}
        placeholder={repsPlaceholder}
        keyboardType="number-pad"
        editable={!props.isReadonly}
        onChangeText={(text) => setDraft((d) => ({ ...d, repsText: text }))}
        onBlur={() => onBlur('reps')}
      />
      <View style={columns.check}>
        {props.isReadonly ? (
          isChecked && <Icon source="check" size={20} color={colors.onPrimaryContainer} />
        ) : (
          <IconButton
            testID="set-row-check"
            icon="check"
            mode="contained"
            selected={isChecked}
            size={20}
            style={{ margin: 0 }}
            onPress={onCheckPress}
          />
        )}
      </View>
    </View>
  );
}
