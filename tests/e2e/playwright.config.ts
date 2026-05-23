import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './specs',
  fullyParallel: false,
  retries: 0,
  use: {
    baseURL: 'http://localhost:4173',
    headless: true,
  },
  webServer: {
    command: 'npx vite preview --port 4173',
    cwd: '../../web',
    url: 'http://localhost:4173',
    reuseExistingServer: true,
    timeout: 15_000,
  },
});
