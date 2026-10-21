import { readFileSync } from "node:fs";
import { defineConfig } from "vite";

export default defineConfig({
  base: "./",
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  plugins: [
    {
      name: "element-index",
      configureServer(server) {
        server.middlewares.use("/element-index.json", (_req, res) => {
          res.setHeader("Content-Type", "application/json");
          res.end(readFileSync("element-index.json"));
        });
      },
      generateBundle() {
        this.emitFile({
          type: "asset",
          fileName: "element-index.json",
          source: readFileSync("element-index.json", "utf8"),
        });
      },
    },
  ],
});
