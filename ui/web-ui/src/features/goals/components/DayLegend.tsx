import { DAY_STATUS_LABEL, type DayStatus } from '../dayStatus';
import { DAY_CELL_CLASSES } from './DayStrip';

const ORDER: DayStatus[] = ['achieved', 'missed', 'pending', 'suppressed', 'no_goal'];

/** Color key for the day strip. */
export function DayLegend() {
  return (
    <ul className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-gray-500">
      {ORDER.map((status) => (
        <li key={status} className="flex items-center gap-1.5">
          <span className={`inline-block h-3 w-3 rounded-sm ${DAY_CELL_CLASSES[status]}`} />
          {DAY_STATUS_LABEL[status]}
        </li>
      ))}
    </ul>
  );
}
