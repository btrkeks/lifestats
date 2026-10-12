import DurationEditor from '@/components/presentation/foundation/editors/duration-editor';
import { spacing } from '@/hooks/useAppTheme';
import { Rest } from '@/models/blueprint-models';
import { Duration } from '@js-joda/core';
import { useTranslate } from '@tolgee/react';
import { useState } from 'react';
import { View } from 'react-native';
import { Dialog } from 'react-native-paper';
import { Portal } from 'react-native-paper';
import Button from '@/components/presentation/foundation/button';
import SegmentedPicker from '@/components/presentation/foundation/segmented-picker';
import { KeyboardAvoidingView } from 'react-native-keyboard-controller';

type ButtonValues = 'short' | 'medium' | 'long' | 'custom';

interface RestEditorDialogProps {
  rest: Rest;
  onRestUpdated: (rest: Rest) => void;
  dialogOpen: boolean;
  setDialogOpen: (open: boolean) => void;
}
export function RestEditorDialog(props: RestEditorDialogProps) {
  const { t } = useTranslate();

  const { onRestUpdated, rest } = props;
  const [buttonValue, setButtonValue] = useState<ButtonValues>(
    (['short', 'medium', 'long'] as const).find(
      (preset) => rest.minRest.equals(Rest[preset].minRest) && rest.maxRest.equals(Rest[preset].maxRest),
    ) ?? 'custom',
  );
  const handleValueChange = (val: ButtonValues) => {
    setButtonValue(val);
    if (val === 'custom') {
    } else {
      onRestUpdated(Rest[val]);
    }
  };

  const updateRest = (type: 'minRest' | 'maxRest') => (duration: Duration) =>
    onRestUpdated({ ...rest, [type]: duration });

  const customView = (
    <View
      style={{
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: spacing[2],
      }}
    >
      <DurationEditor label={t('rest.min.label')} duration={rest.minRest} onDurationUpdated={updateRest('minRest')} />
      <DurationEditor label={t('rest.max.label')} duration={rest.maxRest} onDurationUpdated={updateRest('maxRest')} />
    </View>
  );
  return (
    props.dialogOpen && (
      <Portal>
        <KeyboardAvoidingView
          behavior={'height'}
          style={{
            flex: 1,
            pointerEvents: props.dialogOpen ? 'box-none' : 'none',
          }}
        >
          <Dialog visible={props.dialogOpen} onDismiss={() => props.setDialogOpen(false)}>
            <Dialog.Content>
              <View style={{ width: '100%' }}>
                <SegmentedPicker
                  value={buttonValue}
                  onChange={handleValueChange}
                  equalWidth={false}
                  options={[
                    {
                      value: 'short',
                      label: t('rest.short.label'),
                    },
                    {
                      value: 'medium',
                      label: t('rest.medium.label'),
                    },
                    {
                      value: 'long',
                      label: t('rest.long.label'),
                    },
                    {
                      value: 'custom',
                      label: t('generic.custom.label'),
                    },
                  ]}
                />
              </View>
              {customView}
            </Dialog.Content>
            <Dialog.Actions>
              <Button onPress={() => props.setDialogOpen(false)}>{t('generic.close.button')}</Button>
            </Dialog.Actions>
          </Dialog>
        </KeyboardAvoidingView>
      </Portal>
    )
  );
}
