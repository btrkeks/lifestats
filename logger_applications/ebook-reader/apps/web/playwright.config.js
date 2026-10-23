/* global process */
import { defineConfig } from '@playwright/test';

/** Browser checks of Lifestats sync against `pnpm preview` and an isolated Lifestats server. */
export default defineConfig({
  testDir: 'tests',
  testMatch: '*.e2e.ts',
  workers: 1,
  timeout: 300_000,
  reporter: [['list']],
  use: {
    baseURL: 'http://localhost:4920',
    launchOptions: { executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome-stable' }
  },
  webServer: {
    command: 'pnpm preview --port 4920 --strictPort',
    url: 'http://localhost:4920',
    reuseExistingServer: false
  }
});
