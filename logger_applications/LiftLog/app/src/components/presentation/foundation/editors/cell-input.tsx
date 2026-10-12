import { font, rounding, spacing, useAppTheme } from '@/hooks/useAppTheme';
import { useEffect, useRef, useImperativeHandle, Ref } from 'react';
import { Keyboard, KeyboardTypeOptions, Platform, StyleProp, TextInput, TextStyle } from 'react-native';

interface CellInputProps {
  value: string;
  placeholder?: string | undefined;
  keyboardType: KeyboardTypeOptions;
  editable: boolean;
  onChangeText: (text: string) => void;
  onBlur: () => void;
  testID?: string;
  style?: StyleProp<TextStyle>;
  ref?: Ref<TextInput>;
}

/** A compact, chrome-free numeric cell for table rows; Paper's TextInput is too tall to sit in one. */
export function CellInput({ ref, value, style, ...rest }: CellInputProps) {
  const { colors } = useAppTheme();
  const inputRef = useRef<TextInput>(null);
  useImperativeHandle(ref, () => inputRef.current as TextInput);

  // Android's back gesture hides the keyboard but keeps focus, so the cell would never commit.
  useEffect(() => {
    const subscription = Keyboard.addListener('keyboardDidHide', () => {
      if (inputRef.current?.isFocused()) {
        inputRef.current.blur();
      }
    });
    return () => subscription.remove();
  }, []);

  // When a focused cell blurs, Android hands focus to another cell in the table. Only a cell the user
  // touched selects its text, or that stray focus leaves a selection highlight behind.
  const touched = useRef(false);
  const selectAllOnFocus = () => {
    const wasTouched = touched.current;
    touched.current = false;
    if (Platform.OS === 'android' && value && wasTouched) {
      // Android resets the cursor once the keyboard lays out; selecting after a short delay survives it.
      setTimeout(() => inputRef.current?.setSelection(0, value.length), 50);
    }
  };

  return (
    <TextInput
      {...rest}
      ref={inputRef}
      value={value}
      placeholderTextColor={colors.onSurfaceVariant + '99'}
      onPressIn={() => (touched.current = true)}
      onFocus={selectAllOnFocus}
      selectTextOnFocus={Platform.OS === 'ios'}
      submitBehavior="blurAndSubmit"
      returnKeyType="done"
      textAlign="center"
      style={[
        {
          ...font['text-base'],
          height: spacing[9],
          paddingVertical: 0,
          paddingHorizontal: spacing[1],
          borderRadius: rounding.roundedRectangleRadius,
          backgroundColor: colors.surfaceContainerHighest,
          color: colors.onSurface,
        },
        style,
      ]}
    />
  );
}
