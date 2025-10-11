import type { StatisticMetadata } from '@/types';
import { DefaultGoalEditor } from './DefaultGoalEditor';
import { DailyGoalOverride } from './DailyGoalOverride';

interface GoalEditorModalProps {
  statistic: StatisticMetadata;
  onClose: () => void;
}

export function GoalEditorModal({ statistic, onClose }: GoalEditorModalProps) {
  // Handle clicking on the backdrop to close
  const handleBackdropClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  };

  return (
    <div
      className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4"
      onClick={handleBackdropClick}
    >
      <div className="bg-gray-50 rounded-lg w-full max-w-6xl max-h-[90vh] overflow-y-auto">
        {/* Modal Header */}
        <div className="sticky top-0 bg-white border-b border-gray-200 px-6 py-4 flex items-center justify-between rounded-t-lg">
          <div>
            <h2 className="text-2xl font-bold text-gray-900">{statistic.name}</h2>
            <p className="text-sm text-black mt-1">
              {statistic.category} • {statistic.statistic_type}
            </p>
          </div>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600 transition-colors text-2xl font-bold w-8 h-8 flex items-center justify-center"
            aria-label="Close modal"
          >
            ×
          </button>
        </div>

        {/* Modal Body */}
        <div className="p-6">
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
            <DefaultGoalEditor statisticId={statistic.id} />
            <DailyGoalOverride statisticId={statistic.id} />
          </div>
        </div>
      </div>
    </div>
  );
}
