import Link from 'next/link';
import type { StatisticTypeDefinition } from '@/lib/constants';

type StatisticTypeCardProps = StatisticTypeDefinition;

export function StatisticTypeCard({
  title,
  description,
  icon: Icon,
  href,
}: StatisticTypeCardProps) {
  return (
    <Link
      href={href}
      className="group flex flex-col gap-4 rounded-2xl border border-gray-200 bg-white p-6 shadow-sm transition hover:-translate-y-1 hover:shadow-lg"
    >
      <span className="inline-flex h-12 w-12 items-center justify-center rounded-xl bg-blue-50 text-blue-600">
        <Icon className="h-7 w-7" />
      </span>
      <div>
        <h3 className="text-xl font-semibold text-gray-900">{title}</h3>
        <p className="mt-2 text-sm text-gray-600">{description}</p>
      </div>
      <span className="mt-auto text-sm font-medium text-blue-600 group-hover:text-blue-700">
        Choose this type →
      </span>
    </Link>
  );
}
