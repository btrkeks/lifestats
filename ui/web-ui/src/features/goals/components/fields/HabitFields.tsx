'use client';

import { UseFormRegister, FieldErrors, FieldValues } from 'react-hook-form';

interface HabitFieldsProps {
  register: UseFormRegister<FieldValues>;
  errors: FieldErrors<FieldValues>;
}

/**
 * Form fields specific to Habit goal types.
 * Presentational component - no data fetching, just UI.
 */
export function HabitFields({ register, errors }: HabitFieldsProps) {
  return (
    <div className="flex items-center">
      <input
        id="occurred"
        type="checkbox"
        {...register('occurred')}
        className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
      />
      <label htmlFor="occurred" className="ml-2 block text-sm text-black">
        Goal: Habit Occurred
      </label>
      {errors.occurred && (
        <p className="mt-1 text-sm text-red-600">{String(errors.occurred.message)}</p>
      )}
    </div>
  );
}
