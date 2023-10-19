import * as esbuild from "esbuild";

await esbuild.build({
	bundle: true,
	entryPoints: ["./src/js/runtime.ts"],
	inject: ["./src/js/global.ts"],
	minify: true,
	outfile: "./src/js/runtime.js",
	sourcemap: true,
});
