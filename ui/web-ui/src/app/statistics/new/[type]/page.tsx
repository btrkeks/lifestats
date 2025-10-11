'use client';

import Link from 'next/link';
import { useParams, useRouter } from 'next/navigation';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { STATISTIC_TYPE_DEFINITIONS } from '@/lib/constants';
import { CreateStatisticForm } from '@/features/statistics/components/CreateStatisticForm';
import { createStatistic, getLoggableStatistics } from '@/lib/api';
import type { CreateStatisticRequest, StatisticMetadata, StatisticType } from '@/types';

const DERIVED_TYPES = new Set<StatisticType>(['Summing', 'TimeDifference']);

export default function CreateStatisticPage() {
  const params = useParams<{ type: string }>();
  const router = useRouter();
  const queryClient = useQueryClient();

  const slugParam = Array.isArray(params?.type) ? params.type[0] : params?.type;
  const definition = STATISTIC_TYPE_DEFINITIONS.find((item) => item.slug === slugParam);
  const requiresSources = definition ? DERIVED_TYPES.has(definition.type) : false;

  const loggableQuery = useQuery({
    queryKey: ['loggable-statistics'],
    queryFn: getLoggableStatistics,
    enabled: requiresSources,
  });

  const mutation = useMutation({
    mutationFn: (payload: CreateStatisticRequest) => createStatistic(payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['statistics'] });
      router.push('/dashboard');
    },
  });

  if (!definition) {
    return (
      <div className="min-h-screen bg-gray-50 p-8">
        <div className="mx-auto max-w-xl text-center">
          <h1 className="text-3xl font-semibold text-gray-900">Statistic type not found</h1>
          <p className="mt-3 text-gray-600">
            The requested statistic type is not available. Please choose one from the selector page.
          </p>
          <Link
            href="/statistics/new"
            className="mt-6 inline-block rounded-md bg-blue-600 px-4 py-2 text-white transition hover:bg-blue-700"
          >
            Browse statistic types
          </Link>
        </div>
      </div>
    );
  }

  const loggableStatistics: StatisticMetadata[] = loggableQuery.data ?? [];
  const mutationErrorMessage =
    mutation.isError && mutation.error instanceof Error
      ? mutation.error.message
      : mutation.isError
        ? 'Failed to create statistic.'
        : null;

  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <div className="mx-auto max-w-3xl">
        <Link
          href="/statistics/new"
          className="inline-block text-sm font-medium text-blue-600 transition hover:text-blue-700 hover:underline"
        >
          ← Back to selector
        </Link>

        <div className="mt-6">
          <h1 className="text-4xl font-bold text-gray-900">{definition.title}</h1>
          <p className="mt-2 text-gray-600">{definition.description}</p>
        </div>

        {requiresSources && loggableQuery.isError && (
          <div className="mt-6 rounded-lg border border-red-200 bg-red-50 p-4 text-sm text-red-800">
            We couldn’t load statistics that can be used as sources. Please try again.
          </div>
        )}

        {mutationErrorMessage && (
          <div className="mt-6 rounded-lg border border-red-200 bg-red-50 p-4 text-sm text-red-800">
            {mutationErrorMessage}
          </div>
        )}

        <div className="mt-6 rounded-2xl bg-white p-6 shadow-sm">
          <CreateStatisticForm
            statisticType={definition.type}
            onSubmit={(payload) => mutation.mutate(payload)}
            isSubmitting={mutation.isPending}
            loggableStatistics={loggableStatistics}
            isLoadingSources={loggableQuery.isLoading}
            requiresSources={requiresSources}
          />
        </div>
      </div>
    </div>
  );
}
