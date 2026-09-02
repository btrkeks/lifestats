interface MutationStatusProps {
  error?: Error | null;
  isSuccess?: boolean;
  successText: string;
}

/** Inline feedback line under a form: error message or success confirmation. */
export function MutationStatus({ error, isSuccess = false, successText }: MutationStatusProps) {
  if (error) {
    return <p className="mt-3 text-sm text-rose-600">Error: {error.message}</p>;
  }
  if (isSuccess) {
    return <p className="mt-3 text-sm text-emerald-700">{successText}</p>;
  }
  return null;
}
