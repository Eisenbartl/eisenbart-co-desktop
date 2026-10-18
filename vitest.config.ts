import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  // Serialize DOM integration workers on constrained cloud/Windows hosts.
  // Keep every assertion and per-test deadline; avoid CPU contention between full client workflows.
  test: { maxWorkers: 1, environment: 'jsdom', globals: true, setupFiles: ['./src/test-setup.ts'] },
});
