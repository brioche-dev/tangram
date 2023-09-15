import * as esbuild from "esbuild";

// Run esbuild.
await esbuild.build({
	bundle: true,
	entryPoints: ["main.ts"],
	minify: true,
	outfile: "../../assets/global.js",
	sourcemap: true,
});
