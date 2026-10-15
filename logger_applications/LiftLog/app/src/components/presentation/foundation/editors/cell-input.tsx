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
  /** Enter keeps focus and the keyboard; the caller moves focus or dismisses. */
  onSubmitEditing: () => void;
  returnKeyType: 'next' | 'done';
  testID?: string;
  style?: StyleProp<TextStyle>;
  ref?: Ref<CellInputHandle>;
}

export interface CellInputHandle {
  /** Focuses the cell with its text selected, so typing replaces it. */
  focus: () => void;
}

/** A compact, chrome-free numeric cell for table rows; Paper's TextInput is too tall to sit in one. */
export function CellInput({ ref, value, style, ...rest }: CellInputProps) {
  const { colors } = useAppTheme();
  const inputRef = useRef<TextInput>(null);

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
  // touched or moved to selects its text, or that stray focus leaves a selection highlight behind.
  const selectOnFocus = useRef(false);
  const selectAllOnFocus = () => {
    const shouldSelect = selectOnFocus.current;
    selectOnFocus.current = false;
    if (Platform.OS === 'android' && value && shouldSelect) {
      // Android resets the cursor once the keyboard lays out; selecting after a short delay survives it.
      setTimeout(() => inputRef.current?.setSelection(0, value.length), 50);
    }
  };

  useImperativeHandle(ref, () => ({
    focus: () => {
      // A focused cell gets no focus event, so the flag would wait for the next stray focus.
      if (inputRef.current?.isFocused()) {
        return;
      }
      selectOnFocus.current = true;
      inputRef.current?.focus();
    },
  }));

  return (
    <TextInput
      {...rest}
      ref={inputRef}
      value={value}
      placeholderTextColor={colors.onSurfaceVariant + '99'}
      onPressIn={() => (selectOnFocus.current = true)}
      onFocus={selectAllOnFocus}
      selectTextOnFocus={Platform.OS === 'ios'}
      submitBehavior="submit"
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
