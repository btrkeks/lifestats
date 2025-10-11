import Link from 'next/link';
import { STATISTIC_TYPE_DEFINITIONS } from '@/lib/constants';
import { StatisticTypeCard } from '@/features/statistics/components/StatisticTypeCard';

export default function StatisticTypeSelectorPage() {
  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <div className="mx-auto max-w-5xl">
        <Link
          href="/dashboard"
          className="inline-block text-sm font-medium text-blue-600 transition hover:text-blue-700 hover:underline"
        >
          ← Back to Dashboard
        </Link>

        <div className="mt-6">
          <h1 className="text-4xl font-bold text-gray-900">Create a Statistic</h1>
          <p className="mt-2 max-w-2xl text-gray-600">
            Pick the type of statistic you want to create. We’ll guide you through the details next.
          </p>
        </div>

        <div className="mt-10 grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
          {STATISTIC_TYPE_DEFINITIONS.map((definition) => (
            <StatisticTypeCard key={definition.type} {...definition} />
          ))}
        </div>
      </div>
    </div>
  );
}
