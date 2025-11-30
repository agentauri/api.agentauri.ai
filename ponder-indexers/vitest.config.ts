import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // Test files location (outside src/ to avoid Ponder loading them)
    include: ["__tests__/**/*.test.ts"],

    // Exclude src/ directory from test discovery
    exclude: ["node_modules", "dist", ".ponder", "src"],

    // Global test timeout
    testTimeout: 10000,

    // Run tests in Node.js environment
    environment: "node",

    // Coverage configuration
    coverage: {
      provider: "v8",
      reporter: ["text", "json", "html"],
      exclude: [
        "node_modules/**",
        "dist/**",
        ".ponder/**",
        "**/*.config.ts",
        "**/*.config.js",
      ],
    },
  },
});
