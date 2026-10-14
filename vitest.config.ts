import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  // Bound concurrent DOM workers on Windows CI; preserve per-test deadlines.
  test: { maxWorkers: 2, environment: 'jsdom', globals: true, setupFiles: ['./src/test-setup.ts'] },
});
