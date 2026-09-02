import Link from 'next/link';
import type { ReactNode } from 'react';

/** Page chrome shared by every route: brand link home, primary action, content well. */
export function AppShell({ children }: { children: ReactNode }) {
  return (
    <div className="min-h-screen bg-gray-50 text-gray-900">
      <header className="border-b border-gray-200 bg-white">
        <div className="mx-auto flex max-w-7xl items-center justify-between px-6 py-3">
          <Link href="/" className="text-lg font-bold tracking-tight text-gray-900">
            LifeStats
          </Link>
          <Link
            href="/statistics/new"
            className="rounded-md bg-blue-600 px-3 py-1.5 text-sm font-medium text-white shadow-sm transition hover:bg-blue-700"
          >
            + New statistic
          </Link>
        </div>
      </header>
      <main className="mx-auto max-w-7xl px-6 py-8">{children}</main>
    </div>
  );
}
