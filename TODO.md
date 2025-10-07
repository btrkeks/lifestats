# `TODO.md`

## Frontend: Implement "Dashboard-First" Goal Editing

**Objective:** Replace the current ID-based goal editing page with a user-friendly dashboard. This new interface will display all statistics as interactive cards, allowing users to manage goals in a modal window directly from the context of the statistic they wish to modify.

This refactor improves user experience by removing the need for users to know statistic IDs and creates a more intuitive, centralized management view.

---

### Phase 1: Build the Core Dashboard UI (Read-Only)

#### 1. Create the New Dashboard Page Route

- **Action:** Create the main page file for our new dashboard. This component will act as the "container" or "smart" component, responsible for fetching data and managing the application's state.
- **File:** `src/app/dashboard/page.tsx`
- **Details:**
  - Create a basic functional component that renders a heading like "Statistics Dashboard".
  - This file will be expanded upon in subsequent steps.

#### 2. Create the `StatisticCard` Presentational Component

- **Action:** Create a new, reusable "dumb" component that is only responsible for displaying the data for a single statistic. It will receive all its data and behavior via props.
- **File:** `src/features/statistics/components/StatisticCard.tsx`
- **Contract:** Implement the component with the following props interface and structure:

  ```typescript
  import type { StatisticMetadata } from '@/types';

  interface StatisticCardProps {
    statistic: StatisticMetadata;
    onManageGoal: () => void; // A callback function to be triggered on button click
  }

  export function StatisticCard({ statistic, onManageGoal }: StatisticCardProps) {
    // TODO: Implement the JSX for the card layout.
    // It should display statistic.name and statistic.category.
    // It must include a button that calls `onManageGoal` when clicked.
    return (
      <div className="p-4 bg-white rounded-lg shadow-md">
        <h3>{statistic.name}</h3>
        <p>{statistic.category}</p>
        <button onClick={onManageGoal}>Manage Goal</button>
      </div>
    );
  }
  ```

#### 3. Create the `StatisticGrid` Presentational Component

- **Action:** Create a component that arranges multiple `StatisticCard` components in a grid layout. This component is also "dumb" and simply maps data to child components.
- **File:** `src/features/statistics/components/StatisticGrid.tsx`
- **Contract:** Implement the component to accept a list of statistics and pass the `onManageGoal` callback down to each card.

  ```typescript
  import type { StatisticMetadata } from '@/types';
  import { StatisticCard } from './StatisticCard';

  interface StatisticGridProps {
    statistics: StatisticMetadata[];
    onManageGoal: (statistic: StatisticMetadata) => void;
  }

  export function StatisticGrid({ statistics, onManageGoal }: StatisticGridProps) {
    // TODO: Implement the grid layout (e.g., using Tailwind CSS).
    // Map over the `statistics` array and render a `StatisticCard` for each one.
    // The `key` should be `stat.id`.
    // The `onManageGoal` prop for the card should be `() => onManageGoal(stat)`.
    return (
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        {statistics.map(stat => (
          <StatisticCard
            key={stat.id}
            statistic={stat}
            onManageGoal={() => onManageGoal(stat)}
          />
        ))}
      </div>
    );
  }
  ```

---

### Phase 2: Integrate Data and Interactivity

#### 4. Implement Data Fetching and State Management on the Dashboard Page

- **Action:** Turn the static `DashboardPage` into a dynamic container that fetches live data and prepares for interactivity.
- **File:** `src/app/dashboard/page.tsx`
- **Details:**
  1.  **Data Fetching:** Use the `useQuery` hook from `@tanstack/react-query` to call the `getAllStatistics` API function.
  2.  **State Management:** Introduce a `useState` hook to manage which statistic is currently selected for editing. The initial state should be `null`.
      ```typescript
      const [editingStatistic, setEditingStatistic] =
        useState<StatisticMetadata | null>(null);
      ```
  3.  **Handlers:** Create a handler function `handleOpenModal` that takes a `StatisticMetadata` object and sets it as the `editingStatistic`.
  4.  **Integration:** Render the `StatisticGrid` component, passing the fetched `statistics` data and the `handleOpenModal` handler as props. Implement loading and error states.

#### 5. Create the `GoalEditorModal` Wrapper Component

- **Action:** Create a new component that provides the modal UI and _wraps our existing goal editor components_. This demonstrates the power of component reuse.
- **File:** `src/features/goals/components/GoalEditorModal.tsx`
- **Contract:** The component will receive the selected statistic and a close handler as props.

  ```typescript
  import type { StatisticMetadata } from '@/types';
  import { DefaultGoalEditor } from './DefaultGoalEditor';
  import { DailyGoalOverride } from './DailyGoalOverride';

  interface GoalEditorModalProps {
    statistic: StatisticMetadata;
    onClose: () => void;
  }

  export function GoalEditorModal({ statistic, onClose }: GoalEditorModalProps) {
    // TODO: Implement the modal structure (backdrop, main container, header).
    // The header should contain the statistic name and a close button that calls `onClose`.
    // The modal body should render the *existing* `DefaultGoalEditor` and `DailyGoalOverride`
    // components, passing `statistic.id` to them as a prop.
    return (
      <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center">
        <div className="bg-gray-50 rounded-lg w-full max-w-4xl p-6">
          {/* ... Modal Header ... */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
            <DefaultGoalEditor statisticId={statistic.id} />
            <DailyGoalOverride statisticId={statistic.id} />
          </div>
        </div>
      </div>
    );
  }
  ```

#### 6. Integrate the Modal into the Dashboard Page

- **Action:** Wire up the `GoalEditorModal` to the `DashboardPage` so it appears and disappears based on the `editingStatistic` state.
- **File:** `src/app/dashboard/page.tsx`
- **Details:**
  1.  **Handler:** Create a `handleCloseModal` function that sets `editingStatistic` back to `null`.
  2.  **Conditional Rendering:** Use a conditional expression to render the `GoalEditorModal` only when `editingStatistic` is not `null`.
      ```jsx
      {
        editingStatistic && (
          <GoalEditorModal
            statistic={editingStatistic}
            onClose={handleCloseModal}
          />
        );
      }
      ```

---

### Phase 3: Cleanup and Finalization

#### 7. Update Application Navigation

- **Action:** Update the main landing page to link to our new dashboard instead of the old demo.
- **File:** `src/app/page.tsx`
- **Details:** Find the `<Link>` component pointing to `/demo/goals` and change its `href` to `/dashboard`.

#### 8. Remove Obsolete Code

- **Action:** Delete the old goal management page to keep the codebase clean and avoid confusion.
- **File:** `src/app/demo/goals/page.tsx`
- **Details:** Delete the file and its parent `goals` directory (`src/app/demo/goals/`).

#### 9. Final Review

- **Action:** Run the application and perform a full end-to-end test of the new user flow.
- **Checklist:**
  - [ ] The dashboard loads and displays all statistics correctly.
  - [ ] Clicking "Manage Goal" on a card opens the modal.
  - [ ] The modal correctly displays the goal editors for the selected statistic.
  - [ ] Setting/deleting both default and daily goals works as expected from within the modal.
  - [ ] Closing the modal (either via button or backdrop click) returns the user to the dashboard.
  - [ ] The UI is responsive and looks good on different screen sizes.
