'use client';

import {
  UseFormRegister,
  FieldErrors,
  FieldValues,
  useFieldArray,
  Control,
} from 'react-hook-form';

interface TimeSpansFieldsProps {
  control: Control<FieldValues>;
  register: UseFormRegister<FieldValues>;
  errors: FieldErrors<FieldValues>;
}

/**
 * Dynamic form fields for editing TimeSpans goals.
 * Uses `useFieldArray` so users can add/remove arbitrary time frames.
 */
export function TimeSpansFields({ control, register, errors }: TimeSpansFieldsProps) {
  const { fields, append, remove } = useFieldArray({
    control,
    name: 'target',
  });

  return (
    <>
      <div>
        <h4 className="text-md font-medium text-black">Target Time Frames</h4>
        {fields.map((field, index) => (
          <div key={field.id} className="flex items-center gap-2 mt-2 p-2 border rounded">
            <input
              type="time"
              {...register(`target.${index}.start`)}
              className="w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 px-3 py-2 border text-black"
            />
            <span>to</span>
            <input
              type="time"
              {...register(`target.${index}.end`)}
              className="w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 px-3 py-2 border text-black"
            />
            <button
              type="button"
              onClick={() => remove(index)}
              className="text-red-500 font-bold p-1"
              aria-label={`Remove time frame ${index + 1}`}
            >
              &times;
            </button>
          </div>
        ))}
        <button
          type="button"
          onClick={() => append({ start: '09:00', end: '10:00' })}
          className="mt-2 text-sm text-blue-600 hover:text-blue-800"
        >
          + Add Time Frame
        </button>
        {errors.target && (
          <p className="mt-1 text-sm text-red-600">{String(errors.target.message)}</p>
        )}
      </div>

      <div>
        <label htmlFor="efficiency" className="block text-sm font-medium text-black">
          Target Efficiency (%)
        </label>
        <input
          id="efficiency"
          type="number"
          min="0"
          max="100"
          {...register('efficiency', { valueAsNumber: true })}
          className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 px-3 py-2 border text-black placeholder:text-black"
          placeholder="e.g., 85"
        />
        {errors.efficiency && (
          <p className="mt-1 text-sm text-red-600">{String(errors.efficiency.message)}</p>
        )}
      </div>
    </>
  );
}
