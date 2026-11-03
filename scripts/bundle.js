import { build } from "esbuild";

async function main() {
  await build({
    entryPoints: ["apps/web/src/index.ts"],
    bundle: true,
    outfile: "apps/web/dist/bundle.js",
    format: "esm",
    target: "es2022",
    define: {
      "process.env.NODE_ENV": '"production"',
    },
  });
  console.log("Flux web bundle built successfully.");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
