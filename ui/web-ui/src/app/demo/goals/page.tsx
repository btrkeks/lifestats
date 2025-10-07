'use client';

import { useState } from 'react';
import Link from 'next/link';
import { DefaultGoalEditor } from '@/features/goals/components/DefaultGoalEditor';
import { DailyGoalOverride } from '@/features/goals/components/DailyGoalOverride';

export default function GoalsDemoPage() {
  const [statisticId, setStatisticId] = useState<string>('1');

  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <div className="max-w-6xl mx-auto">
        <div className="mb-8">
          <Link
            href="/"
            className="text-blue-600 hover:text-blue-800 hover:underline"
          >
            ← Back to Home
          </Link>
        </div>

        <header className="mb-8">
          <h1 className="text-4xl font-bold text-gray-900 mb-2">Goal Management Demo</h1>
          <p className="text-gray-600">
            Manage default goals and daily overrides for your statistics
          </p>
        </header>

        <div className="mb-6 bg-white rounded-lg shadow-md p-6">
          <label htmlFor="statisticId" className="block text-sm font-medium text-gray-700 mb-2">
            Select Statistic ID
          </label>
          <input
            id="statisticId"
            type="number"
            value={statisticId}
            onChange={(e) => setStatisticId(e.target.value)}
            className="w-full max-w-xs rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 px-3 py-2 border"
            placeholder="Enter statistic ID"
            min="1"
          />
          <p className="mt-2 text-sm text-gray-500">
            Note: Make sure the statistic exists in your database. You can create one via the API.
          </p>
        </div>

        <div className="grid lg:grid-cols-2 gap-8">
          <DefaultGoalEditor statisticId={parseInt(statisticId) || 1} />
          <DailyGoalOverride statisticId={parseInt(statisticId) || 1} />
        </div>

        <div className="mt-8 bg-blue-50 border border-blue-200 rounded-lg p-6">
          <h3 className="text-lg font-semibold text-blue-900 mb-3">How to use this demo:</h3>
          <ol className="list-decimal list-inside space-y-2 text-blue-800">
            <li>Make sure the backend API is running on <code className="bg-blue-100 px-1 rounded">http://localhost:8080</code></li>
            <li>Enter a valid statistic ID (or create one using the API first)</li>
            <li>Set a default goal that applies to all days</li>
            <li>Optionally set daily overrides for specific dates</li>
            <li>Daily overrides take precedence over default goals</li>
          </ol>
        </div>

        <div className="mt-6 bg-gray-100 border border-gray-300 rounded-lg p-6">
          <h3 className="text-lg font-semibold text-gray-900 mb-3">API Endpoints Used:</h3>
          <ul className="space-y-1 text-sm text-gray-700 font-mono">
            <li>GET /statistics/:id/goals/default</li>
            <li>PUT /statistics/:id/goals/default</li>
            <li>DELETE /statistics/:id/goals/default</li>
            <li>GET /statistics/:id/goals/:date</li>
            <li>PUT /statistics/:id/goals/:date</li>
            <li>DELETE /statistics/:id/goals/:date</li>
          </ul>
        </div>
      </div>
    </div>
  );
}
