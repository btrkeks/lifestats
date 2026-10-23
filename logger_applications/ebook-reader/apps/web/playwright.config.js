/* global process */
import { defineConfig } from '@playwright/test';

const port = process.env.LIFESTATS_E2E_PREVIEW_PORT || '4920';

/** Browser checks of Lifestats sync against `pnpm preview` and an isolated Lifestats server. */
export default defineConfig({
  testDir: 'tests',
  testMatch: '*.e2e.ts',
  workers: 1,
  timeout: 300_000,
  reporter: [['list']],
  use: {
    baseURL: `http://localhost:${port}`,
    launchOptions: { executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome-stable' }
  },
  webServer: {
    command: `pnpm preview --port ${port} --strictPort`,
    url: `http://localhost:${port}`,
    reuseExistingServer: false
  }
});
