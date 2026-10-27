'use client';

import type { ReactNode } from 'react';
import { cn } from 'cn';
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group';

export interface SegmentedOption<T extends string> {
  value: T;
  label: ReactNode;
}

/** A single-choice segmented control (one option is always on). */
export function Segmented<T extends string>({
  value,
  onChange,
  options,
  label,
  className,
}: {
  value: T;
  onChange: (value: T) => void;
  options: SegmentedOption<T>[];
  label: string;
  className?: string;
}) {
  return (
    <ToggleGroup
      aria-label={label}
      value={[value]}
      onValueChange={(next) => {
        const picked = options.find(
          (option) => next.includes(option.value) && option.value !== value
        );
        if (picked) onChange(picked.value);
      }}
      variant="segmented"
      size="sm"
      spacing={0.5}
      className={cn('w-full rounded-lg bg-line-2 p-[3px]', className)}
    >
      {options.map((option) => (
        <ToggleGroupItem key={option.value} value={option.value} className="h-7 text-[12.5px]">
          {option.label}
        </ToggleGroupItem>
      ))}
    </ToggleGroup>
  );
}
