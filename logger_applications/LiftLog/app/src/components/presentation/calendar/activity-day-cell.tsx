import { SurfaceText } from '@/components/presentation/foundation/surface-text';
import TouchableRipple from '@/components/presentation/foundation/touchable-ripple';
import { useAppTheme } from '@/hooks/useAppTheme';
import { useFormatDate } from '@/hooks/useFormatDate';
import { ActivityCell } from '@/store/activity';
import { memo } from 'react';
import { View } from 'react-native';
import { levelColor } from '@/components/presentation/calendar/activity-colors';

interface ActivityDayCellProps {
  cell: ActivityCell;
  isSelected: boolean;
  onPress?: (cell: ActivityCell) => void;
}

export const ActivityDayCell = memo(function ActivityDayCell({ cell, isSelected, onPress }: ActivityDayCellProps) {
  const { colors } = useAppTheme();
  const formatDate = useFormatDate();

  const { background, foreground } = levelColor(cell.level, colors);
  const isEmpty = cell.level === 0;
  const outlineToday = cell.isToday && isEmpty;

  return (
    <View style={{ flex: 1, aspectRatio: 1, opacity: cell.isOutsideFocus ? 0.4 : 1 }}>
      <TouchableRipple
        onPress={onPress && !cell.isFuture ? () => onPress(cell) : undefined}
        disabled={cell.isFuture}
        style={{ flex: 1, borderRadius: 1000, overflow: 'hidden' }}
      >
        <View
          style={{
            flex: 1,
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: 1000,
            backgroundColor: background,
            borderColor: isSelected ? colors.primary : outlineToday ? colors.outline : 'transparent',
            borderWidth: isSelected ? 2 : 1,
          }}
        >
          <SurfaceText font="text-sm" style={isEmpty ? undefined : { color: foreground }}>
            {formatDate(cell.date, { day: 'numeric' })}
          </SurfaceText>
        </View>
      </TouchableRipple>
    </View>
  );
});
