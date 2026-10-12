import { SurfaceText } from '@/components/presentation/foundation/surface-text';
import { rounding, spacing, useAppTheme } from '@/hooks/useAppTheme';
import { useOwnHistory } from '@/components/smart/own-history-provider';
import { useTranslate } from '@tolgee/react';
import { View } from 'react-native';
import { Icon } from 'react-native-paper';

export function HistoryPrBadges({ sessionId }: { sessionId: string }) {
  const records = useOwnHistory().personalRecords.get(sessionId);
  const { t } = useTranslate();
  const { colors } = useAppTheme();

  if (!records?.length) {
    return null;
  }

  return (
    <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: spacing[1] }}>
      {records.map((record) => (
        <View
          key={record.exerciseName}
          style={{
            flexDirection: 'row',
            alignItems: 'center',
            gap: spacing[1],
            paddingVertical: spacing[1],
            paddingHorizontal: spacing[2],
            borderRadius: rounding.roundedRectangleRadius,
            backgroundColor: colors.tertiaryContainer,
          }}
        >
          <Icon source="trendingUp" size={14} color={colors.onTertiaryContainer} />
          <SurfaceText font="text-xs" color="onTertiaryContainer">
            {t('history.pr_badge.label', { exercise: record.exerciseName })}
          </SurfaceText>
        </View>
      ))}
    </View>
  );
}
