'use client';

import { UseFormRegister, FieldErrors, FieldValues } from 'react-hook-form';

interface PointInTimeFieldsProps {
  register: UseFormRegister<FieldValues>;
  errors: FieldErrors<FieldValues>;
}

/**
 * Form fields specific to PointInTime goal types.
 * Presentational component - no data fetching, just UI.
 */
export function PointInTimeFields({ register, errors }: PointInTimeFieldsProps) {
  return (
    <div>
      <label htmlFor="deadline" className="block text-sm font-medium text-black">
        Deadline Time
      </label>
      <input
        id="deadline"
        type="time"
        step="1"
        {...register('deadline', {
          required: 'Deadline is required',
        })}
        className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 px-3 py-2 border text-black"
      />
      {errors.deadline && (
        <p className="mt-1 text-sm text-red-600">{String(errors.deadline.message)}</p>
      )}
      <p className="mt-1 text-xs text-black">Format: HH:MM:SS</p>
    </div>
  );
}
