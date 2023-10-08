import * as esbuild from "esbuild";

await esbuild.build({
	bundle: true,
	entryPoints: ["main.ts"],
	minify: true,
	outfile: "../../../assets/runtime.js",
	sourcemap: true,
});
