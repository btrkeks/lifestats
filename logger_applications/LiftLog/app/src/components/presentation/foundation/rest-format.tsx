import React from 'react';
import { TextProps } from 'react-native';
import { RestWindow } from '@/models/session-models';
import { formatTimeSpan } from '@/utils/format-time-span';
import { Text } from 'react-native-paper';

interface RestFormatProps {
  rest: RestWindow;
}

export default function RestFormat({ rest }: RestFormatProps & TextProps) {
  return (
    <Text>
      {rest.minRest.compareTo(rest.maxRest) >= 0
        ? formatTimeSpan(rest.minRest)
        : `${formatTimeSpan(rest.minRest)} - ${formatTimeSpan(rest.maxRest)}`}
    </Text>
  );
}

export { formatTimeSpan };
