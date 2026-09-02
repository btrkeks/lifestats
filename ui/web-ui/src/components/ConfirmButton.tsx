'use client';

import { useEffect, useState } from 'react';

interface ConfirmButtonProps {
  label: string;
  confirmLabel?: string;
  onConfirm: () => void;
  disabled?: boolean;
  className?: string;
}

/**
 * Two-step destructive action: the first click arms the button, a second click
 * within a few seconds confirms. Avoids blocking `confirm()` dialogs.
 */
export function ConfirmButton({
  label,
  confirmLabel = 'Click again to confirm',
  onConfirm,
  disabled = false,
  className = '',
}: ConfirmButtonProps) {
  const [armed, setArmed] = useState(false);

  useEffect(() => {
    if (!armed) return;
    const timer = setTimeout(() => setArmed(false), 4000);
    return () => clearTimeout(timer);
  }, [armed]);

  const handleClick = () => {
    if (armed) {
      setArmed(false);
      onConfirm();
    } else {
      setArmed(true);
    }
  };

  return (
    <button
      type="button"
      onClick={handleClick}
      disabled={disabled}
      aria-pressed={armed}
      className={`${className} ${armed ? 'ring-2 ring-rose-400 ring-offset-1' : ''}`}
    >
      {armed ? confirmLabel : label}
    </button>
  );
}
