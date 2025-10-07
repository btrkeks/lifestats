# `TODO.md`

This document outlines the implementation plan for the new Web UI, focusing first on the required API enhancements and then on the frontend application itself.

## Phase 1: Enhance the Backend Web API

Our first priority is to create a robust and comprehensive JSON API within the `lifestats-web-api` crate. This API will serve as the single source of truth for the new frontend, ensuring a clean separation of concerns.

### `lifestats-application`: New Use Cases

The existing services are query- and command-oriented, which is excellent. We need to add capabilities for creating and managing the `Statistic` entities themselves.

- **1. Implement `StatisticManagementService`:**
  - **Action:** In `core/application/src/use_cases/`, create a new module `statistic_management_service.rs`.
  - **Content:** This service will handle the creation, updating, and deletion of statistic metadata.
  - **Methods:**
    - `create_statistic(command: CreateStatisticCommand) -> ApplicationResult<StatisticMetadata>`
    - `update_statistic(command: UpdateStatisticCommand) -> ApplicationResult<()>`
    - `delete_statistic(command: DeleteStatisticCommand) -> ApplicationResult<()>`
  - **Rationale:** This encapsulates the business logic for managing statistics in a dedicated use case, keeping it separate from logging or querying. It continues our pattern of command-oriented services.
  - **Commands:** Define the `CreateStatisticCommand`, `UpdateStatisticCommand`, and `DeleteStatisticCommand` structs. These will contain primitives like `name: String`, `category: String`, `statistic_type: String`, etc., which the service will translate into domain objects.

### `lifestats-web-api`: New API Endpoints

We will now expose the new use cases and existing goal management features via RESTful endpoints.

- **1. Create Statistic Management Endpoints:**
  - **Action:** In `core/infrastructure/web_api/src/routes/statistics.rs`, add the following handlers.
  - **Endpoints:**
    - `POST /statistics` -> `create_statistic()`: Accepts a JSON body, validates it, creates a `CreateStatisticCommand`, calls the `StatisticManagementService`, and returns `201 Created` with the new statistic's metadata.
    - `PUT /statistics/:id` -> `update_statistic()`: Accepts a JSON body for updating a statistic's metadata, creates an `UpdateStatisticCommand`, and returns `204 No Content`.
    - `DELETE /statistics/:id` -> `delete_statistic()`: Creates a `DeleteStatisticCommand` and returns `204 No Content`.
  - **Rationale:** These endpoints provide the necessary CRUD operations for the UI to manage the list of available statistics.

- **2. Create Comprehensive Goal Management Endpoints:**
  - **Action:** In `core/infrastructure/web_api/src/routes/`, create a new `goals.rs` module and wire it into the router.
  - **Endpoints:**
    - `GET /statistics/:id/goals/default` -> `get_default_goal()`: Fetches and returns the current default goal for a statistic.
    - `PUT /statistics/:id/goals/default` -> `set_default_goal()`: Accepts a JSON body with goal data (e.g., `{ "target": 100 }`), calls the `GoalManagementService`, and returns `204 No Content`.
    - `DELETE /statistics/:id/goals/default` -> `delete_default_goal()`: Clears the default goal for a statistic.
    - `GET /statistics/:id/goals/:date` -> `get_daily_goal()`: Fetches a specific goal override for a given date.
    - `PUT /statistics/:id/goals/:date` -> `set_daily_goal()`: Sets or updates a goal override for a specific date.
    - `DELETE /statistics/:id/goals/:date` -> `delete_daily_goal()`: Deletes a goal override for a specific date.
  - **Rationale:** The UI needs granular control over both default and daily goals. Exposing these actions via a RESTful API makes the frontend logic simple and declarative.

- **3. Create Data Transfer Objects (DTOs):**
  - **Action:** In `core/infrastructure/web_api/src/dto/`, create `statistic.rs` and `goal.rs` to define the JSON request/response structures for the new endpoints.
  - **Content:** Use `serde` for serialization and `validator` for incoming request validation. This keeps validation logic at the boundary of the system, protecting the application services from invalid data.
  - **Example:**
    ```rust
    // In dto/statistic.rs
    #[derive(Deserialize, Validate)]
    pub struct CreateStatisticRequest {
        #[validate(length(min = 1))]
        pub name: String,
        #[validate(length(min = 1))]
        pub category: String,
        pub statistic_type: String, // e.g., "IntegerAmount", "TimeSpans"
    }
    ```
  - **Rationale:** DTOs are a critical part of the Adapter pattern. They decouple the API's public contract from the internal domain models, allowing them to evolve independently.

---

## Phase 2: Frontend SPA Implementation

With a capable API in place, we can now build the frontend. This will live in a new top-level directory, `ui/`, sibling to `core/` and `docs/`.

- **1. Initialize the Next.js Project:**
  - **Action:** Use the official Next.js setup tool to create a new project with TypeScript and Tailwind CSS.
    ```bash
    npx create-next-app@latest ui
    # When prompted, choose:
    # - TypeScript: Yes
    # - ESLint: Yes
    # - Tailwind CSS: Yes
    # - `src/` directory: Yes
    # - App Router: Yes
    ```
  - **Rationale:** This command scaffolds a complete, production-ready frontend project with all the recommended tooling, saving significant setup time.

- **2. Structure the Frontend Project:**
  - **Action:** Inside `ui/src/`, create the following directory structure.
    ```
    ui/src/
    ├── app/              # Next.js App Router pages
    ├── components/       # Reusable UI components (buttons, inputs, etc.)
    ├── features/         # Components and hooks for specific features
    │   └── goals/
    │       ├── components/  # e.g., GoalEditorForm.tsx
    │       └── hooks/       # e.g., useGoals.ts (data fetching logic)
    ├── lib/              # API client, utility functions
    └── types/            # TypeScript type definitions
    ```
  - **Rationale:** A feature-sliced structure is highly maintainable. It co-locates all the code related to a single feature (like "goal management"), making it easier to understand and modify than a role-based structure (e.g., all components in one folder, all hooks in another).

- **3. Implement the API Client:**
  - **Action:** In `ui/src/lib/api.ts`, create a simple, typed API client using `fetch` or a library like `axios`. Define functions that correspond to our new backend endpoints.
  - **Action:** In `ui/src/types/index.ts`, define TypeScript interfaces that match the API's DTOs (e.g., `StatisticMetadata`, `Goal`).
  - **Rationale:** Centralizing API communication makes the code DRY and easy to debug. Typing the responses ensures that the rest of the application knows what data shape to expect from the backend.

- **4. Implement the Goals Management Page:**
  - **Action:** Create the main page component in `ui/src/app/statistics/[id]/goals/page.tsx`.
  - **Action:** In `ui/src/features/goals/hooks/useGoals.ts`, use **TanStack Query** (`useQuery`, `useMutation`) to fetch and update goal data. This hook will encapsulate all the logic for interacting with the goals API.
  - **Action:** In `ui/src/features/goals/components/`, build the UI components:
    - `DefaultGoalEditor.tsx`: A form to view and edit the default goal.
    - `DailyGoalOverride.tsx`: A calendar or date picker to select a day and a form to set a goal for that day.
    - `GoalList.tsx`: A component to display existing daily overrides.
  - **Rationale:** Using TanStack Query will drastically simplify state management by handling caching, loading states, and error states automatically. Building small, focused components makes the UI easier to test and reason about.
