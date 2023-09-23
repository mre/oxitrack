import { dirname, resolve } from "path";
import { fileURLToPath } from "url";

const dir = dirname(fileURLToPath(import.meta.url));

export default {
  entry: {
    stats: "./ts/stats.ts",
  },
  output: {
    path: resolve(dir, "static"),
    library: ["oxitraffic", "[name]"],
  }
};
