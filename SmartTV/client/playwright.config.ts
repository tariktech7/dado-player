import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "tests/ui",
  fullyParallel: false,
  workers: 1,
  timeout: 30000,
  use: {
    channel: process.env.DADO_BROWSER_CHANNEL || undefined,
    baseURL: "http://127.0.0.1:18497",
    viewport: { width: 1920, height: 1080 },
    headless: true,
  },
  webServer: {
    command: "npm run serve",
    url: "http://127.0.0.1:18497",
    reuseExistingServer: !process.env.CI,
  },
  reporter: "list",
});
