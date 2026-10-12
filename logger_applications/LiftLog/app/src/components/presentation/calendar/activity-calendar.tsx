import { SurfaceText } from '@/components/presentation/foundation/surface-text';
import { spacing } from '@/hooks/useAppTheme';
import { useFormatDate } from '@/hooks/useFormatDate';
import { ActivityCell, ActivityRow } from '@/store/activity';
import { getDateOnDay } from '@/utils/format-date';
import { DayOfWeek, LocalDate } from '@js-joda/core';
import { ReactNode } from 'react';
import { I18nManager, View, ViewStyle } from 'react-native';
import { ActivityDayCell } from '@/components/presentation/calendar/activity-day-cell';

interface ActivityCalendarProps {
  rows: ActivityRow[];
  firstDayOfWeek: DayOfWeek;
  selectedDate?: LocalDate;
  header?: ReactNode;
  footer?: ReactNode;
  onCellPress?: (cell: ActivityCell) => void;
}

export function ActivityCalendar({
  rows,
  firstDayOfWeek,
  selectedDate,
  header,
  footer,
  onCellPress,
}: ActivityCalendarProps) {
  const formatDate = useFormatDate();

  const headerDates = Array.from({ length: 7 }, (_, offset) =>
    getDateOnDay(DayOfWeek.of(((firstDayOfWeek.ordinal() + offset) % 7) + 1)),
  );

  return (
    <View style={{ alignItems: 'stretch', gap: spacing[1] }}>
      {header}

      <ForceLTRRow style={{ gap: spacing[1] }}>
        {headerDates.map((date, index) => (
          <SurfaceText
            key={index}
            font="text-xs"
            color="onSurfaceVariant"
            style={{ flex: 1, textAlign: 'center', letterSpacing: 0.6 }}
          >
            {formatDate(date, { weekday: 'narrow' }).toUpperCase()}
          </SurfaceText>
        ))}
      </ForceLTRRow>

      {rows.map((row) => (
        <ForceLTRRow key={row.key} style={{ gap: spacing[1] }}>
          {row.cells.map((cell, columnIndex) => (
            // Position, not date: keying by date would remount all 42 cells on every month change.
            <ActivityDayCell
              key={columnIndex}
              cell={cell}
              isSelected={!!selectedDate?.isEqual(cell.date)}
              onPress={onCellPress}
            />
          ))}
        </ForceLTRRow>
      ))}

      {footer}
    </View>
  );
}

function ForceLTRRow({ children, style }: { children: ReactNode; style?: ViewStyle }) {
  return <View style={[{ flexDirection: I18nManager.isRTL ? 'row-reverse' : 'row' }, style]}>{children}</View>;
}
