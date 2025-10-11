'use client';

import { UseFormRegister, FieldErrors, FieldValues } from 'react-hook-form';

interface IntegerAmountFieldsProps {
  register: UseFormRegister<FieldValues>;
  errors: FieldErrors<FieldValues>;
}

/**
 * Form fields specific to IntegerAmount and Summing goal types.
 * Presentational component - no data fetching, just UI.
 */
export function IntegerAmountFields({ register, errors }: IntegerAmountFieldsProps) {
  return (
    <>
      <div>
        <label htmlFor="target" className="block text-sm font-medium text-black">
          Target Value
        </label>
        <input
          id="target"
          type="number"
          {...register('target', {
            required: 'Target value is required',
            valueAsNumber: true,
          })}
          className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 px-3 py-2 border text-black placeholder:text-black"
          placeholder="e.g., 100"
        />
        {errors.target && (
          <p className="mt-1 text-sm text-red-600">{String(errors.target.message)}</p>
        )}
      </div>

      <div className="flex items-center">
        <input
          id="is_at_least"
          type="checkbox"
          {...register('is_at_least')}
          className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
        />
        <label htmlFor="is_at_least" className="ml-2 block text-sm text-black">
          Is At Least (vs. At Most)
        </label>
      </div>
    </>
  );
}
