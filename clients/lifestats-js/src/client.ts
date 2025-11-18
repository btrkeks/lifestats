/**
 * LifeStats API Client
 * Lean, fetch-based client for the LifeStats API
 */

import {
  StatisticMetadata,
  CreateStatisticRequest,
  LogValueRequest,
  LogTimeFrameRequest,
  SetDefaultGoalRequest,
  SetDailyGoalRequest,
  GoalResponse,
  DateString,
  LifeStatsError,
} from './types';
import {
  statisticMetadataSchema,
  createStatisticRequestSchema,
  logValueRequestSchema,
  logTimeFrameRequestSchema,
  setDefaultGoalRequestSchema,
  setDailyGoalRequestSchema,
  goalResponseSchema,
} from './schemas';

export interface ClientOptions {
  baseUrl?: string;
  headers?: Record<string, string>;
  /** Validate responses with Zod schemas (default: true) */
  validateResponses?: boolean;
}

export class LifeStatsClient {
  private baseUrl: string;
  private headers: Record<string, string>;
  private validateResponses: boolean;

  constructor(options: ClientOptions = {}) {
    this.baseUrl = options.baseUrl || 'http://localhost:3000';
    this.headers = {
      'Content-Type': 'application/json',
      ...options.headers,
    };
    this.validateResponses = options.validateResponses ?? true;
  }

  // ==========================================================================
  // Private Helpers
  // ==========================================================================

  private async request<T>(
    endpoint: string,
    options?: RequestInit,
  ): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;
    const response = await fetch(url, {
      ...options,
      headers: {
        ...this.headers,
        ...options?.headers,
      },
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new LifeStatsError(
        `HTTP ${response.status}: ${errorText}`,
        response.status,
        errorText,
      );
    }

    // Handle empty responses (204 No Content)
    if (response.status === 204) {
      return undefined as T;
    }

    return response.json();
  }

  // ==========================================================================
  // Statistics API
  // ==========================================================================

  /**
   * Get all statistics
   */
  async getAllStatistics(): Promise<StatisticMetadata[]> {
    const data = await this.request<StatisticMetadata[]>('/statistics');
    if (this.validateResponses) {
      return data.map((stat) => statisticMetadataSchema.parse(stat));
    }
    return data;
  }

  /**
   * Get a single statistic by ID
   */
  async getStatistic(id: number): Promise<StatisticMetadata> {
    const data = await this.request<StatisticMetadata>(`/statistics/${id}`);
    if (this.validateResponses) {
      return statisticMetadataSchema.parse(data);
    }
    return data;
  }

  /**
   * Create a new statistic
   */
  async createStatistic(
    request: CreateStatisticRequest,
  ): Promise<StatisticMetadata> {
    if (this.validateResponses) {
      createStatisticRequestSchema.parse(request);
    }
    const data = await this.request<StatisticMetadata>('/statistics', {
      method: 'POST',
      body: JSON.stringify(request),
    });
    if (this.validateResponses) {
      return statisticMetadataSchema.parse(data);
    }
    return data;
  }

  /**
   * Delete a statistic
   */
  async deleteStatistic(id: number): Promise<void> {
    await this.request<void>(`/statistics/${id}`, {
      method: 'DELETE',
    });
  }

  /**
   * Log a value to a statistic
   */
  async logValue(
    statisticId: number,
    date: DateString,
    request: LogValueRequest,
  ): Promise<void> {
    if (this.validateResponses) {
      logValueRequestSchema.parse(request);
    }
    await this.request<void>(`/statistics/${statisticId}/log/${date}`, {
      method: 'POST',
      body: JSON.stringify(request),
    });
  }

  /**
   * Log a time frame to a TimeSpans statistic
   */
  async logTimeFrame(
    statisticId: number,
    date: DateString,
    request: LogTimeFrameRequest,
  ): Promise<void> {
    if (this.validateResponses) {
      logTimeFrameRequestSchema.parse(request);
    }
    await this.request<void>(
      `/statistics/${statisticId}/log_time_frame/${date}`,
      {
        method: 'POST',
        body: JSON.stringify(request),
      },
    );
  }

  // ==========================================================================
  // Goals API
  // ==========================================================================

  /**
   * Get default goal for a statistic
   * Returns null if no default goal is set
   */
  async getDefaultGoal(statisticId: number): Promise<GoalResponse | null> {
    try {
      const data = await this.request<GoalResponse>(
        `/statistics/${statisticId}/goals/default`,
      );
      if (this.validateResponses) {
        return goalResponseSchema.parse(data) as GoalResponse;
      }
      return data;
    } catch (error) {
      if (error instanceof LifeStatsError && error.statusCode === 404) {
        return null;
      }
      throw error;
    }
  }

  /**
   * Set default goal for a statistic
   */
  async setDefaultGoal(
    statisticId: number,
    request: SetDefaultGoalRequest,
  ): Promise<void> {
    if (this.validateResponses) {
      setDefaultGoalRequestSchema.parse(request);
    }
    await this.request<void>(`/statistics/${statisticId}/goals/default`, {
      method: 'PUT',
      body: JSON.stringify(request),
    });
  }

  /**
   * Delete default goal for a statistic
   */
  async deleteDefaultGoal(statisticId: number): Promise<void> {
    await this.request<void>(`/statistics/${statisticId}/goals/default`, {
      method: 'DELETE',
    });
  }

  /**
   * Get daily goal for a statistic on a specific date
   * Returns null if no daily goal is set
   */
  async getDailyGoal(
    statisticId: number,
    date: DateString,
  ): Promise<GoalResponse | null> {
    try {
      const data = await this.request<GoalResponse>(
        `/statistics/${statisticId}/goals/${date}`,
      );
      if (this.validateResponses) {
        return goalResponseSchema.parse(data) as GoalResponse;
      }
      return data;
    } catch (error) {
      if (error instanceof LifeStatsError && error.statusCode === 404) {
        return null;
      }
      throw error;
    }
  }

  /**
   * Set daily goal for a statistic on a specific date
   * Can either set a goal or suppress the default goal
   */
  async setDailyGoal(
    statisticId: number,
    date: DateString,
    request: SetDailyGoalRequest,
  ): Promise<void> {
    if (this.validateResponses) {
      setDailyGoalRequestSchema.parse(request);
    }
    await this.request<void>(`/statistics/${statisticId}/goals/${date}`, {
      method: 'PUT',
      body: JSON.stringify(request),
    });
  }

  /**
   * Delete daily goal for a statistic on a specific date
   */
  async deleteDailyGoal(statisticId: number, date: DateString): Promise<void> {
    await this.request<void>(`/statistics/${statisticId}/goals/${date}`, {
      method: 'DELETE',
    });
  }
}
