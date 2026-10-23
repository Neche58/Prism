import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // Only run the TypeScript sources. Without this, a local `tsc` build leaves
    // compiled copies in `dist/` that vitest would pick up and run a second time.
    include: ["src/**/*.test.ts"],
  },
});
