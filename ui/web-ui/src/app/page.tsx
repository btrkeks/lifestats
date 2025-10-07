import Link from 'next/link';

export default function Home() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100 p-8">
      <div className="max-w-4xl mx-auto">
        <header className="text-center mb-12">
          <h1 className="text-5xl font-bold text-gray-900 mb-4">LifeStats</h1>
          <p className="text-xl text-gray-600">
            Personal observability platform for tracking habits and wellness
          </p>
        </header>

        <div className="grid md:grid-cols-2 gap-6 mb-12">
          <div className="bg-white rounded-lg shadow-lg p-6 hover:shadow-xl transition-shadow">
            <h2 className="text-2xl font-semibold text-gray-800 mb-3">Goal Management</h2>
            <p className="text-gray-600 mb-4">
              Set and manage default goals or daily overrides for your statistics
            </p>
            <Link
              href="/dashboard"
              className="inline-block px-6 py-3 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
            >
              Open Dashboard
            </Link>
          </div>

          <div className="bg-white rounded-lg shadow-lg p-6 hover:shadow-xl transition-shadow">
            <h2 className="text-2xl font-semibold text-gray-800 mb-3">API Documentation</h2>
            <p className="text-gray-600 mb-4">
              Explore the REST API endpoints for statistic and goal management
            </p>
            <a
              href="http://localhost:8080/api-docs"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-block px-6 py-3 bg-indigo-600 text-white rounded-md hover:bg-indigo-700 transition-colors"
            >
              View API Docs
            </a>
          </div>
        </div>

        <div className="bg-white rounded-lg shadow-lg p-6">
          <h2 className="text-2xl font-semibold text-gray-800 mb-4">Available Features</h2>
          <ul className="space-y-3 text-gray-700">
            <li className="flex items-start">
              <span className="text-green-600 mr-2">✓</span>
              <span><strong>Statistic Management:</strong> Create, read, and delete statistics (6 types supported)</span>
            </li>
            <li className="flex items-start">
              <span className="text-green-600 mr-2">✓</span>
              <span><strong>Goal System:</strong> Set default goals and daily overrides with flexible goal_data</span>
            </li>
            <li className="flex items-start">
              <span className="text-green-600 mr-2">✓</span>
              <span><strong>REST API:</strong> Full CRUD operations with validation and error handling</span>
            </li>
            <li className="flex items-start">
              <span className="text-green-600 mr-2">✓</span>
              <span><strong>Modern Frontend:</strong> Next.js 15 with TypeScript, TanStack Query, and Tailwind CSS</span>
            </li>
          </ul>
        </div>

        <footer className="mt-12 text-center text-gray-600">
          <p>Built with Hexagonal Architecture • Rust + Next.js</p>
        </footer>
      </div>
    </div>
  );
}
