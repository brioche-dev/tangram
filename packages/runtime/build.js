import * as esbuild from "esbuild";

await esbuild.build({
	bundle: true,
	entryPoints: ["./src/js/main.ts"],
	minify: true,
	outfile: process.env["OUT_DIR"] + "/main.js",
	sourcemap: true,
});
