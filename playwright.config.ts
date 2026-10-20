import { defineConfig, devices } from '@playwright/test';

const port = 1438;
const baseURL = `http://127.0.0.1:${port}`;

export default defineConfig({
  testDir: 'src/tests/e2e',
  fullyParallel: true,
  reporter: 'list',
  use: { ...devices['Desktop Chrome'], baseURL, trace: 'on-first-retry' },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: `npm run dev -- --host 127.0.0.1 --port ${port} --strictPort`,
    env: { CHOKIDAR_USEPOLLING: 'true' },
    url: baseURL,
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
