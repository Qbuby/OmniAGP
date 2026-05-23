import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './specs',
  fullyParallel: false,
  retries: 0,
  use: {
    baseURL: 'http://localhost:8080',
    headless: true,
  },
});
