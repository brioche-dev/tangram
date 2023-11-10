import * as esbuild from "esbuild";

await esbuild.build({
	bundle: true,
	entryPoints: ["./src/js/main.ts"],
	minify: true,
	outfile: "./src/js/main.js",
	sourcemap: true,
});
