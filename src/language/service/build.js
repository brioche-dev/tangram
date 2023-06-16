import * as esbuild from "esbuild";
import alias from "esbuild-plugin-alias";
import * as path from "path";

// Run esbuild.
await esbuild.build({
	bundle: true,
	entryPoints: ["main.ts"],
	inject: ["node_global.js"],
	minify: true,
	outfile: "../../../assets/language_service.js",
	plugins: [
		alias({
			assert: path.resolve("./node_builtins/assert.cjs"),
			crypto: path.resolve("./node_builtins/crypto.cjs"),
			events: path.resolve("./node_builtins/events.cjs"),
			fs: path.resolve("./node_builtins/fs.cjs"),
			module: path.resolve("./node_builtins/module.cjs"),
			os: path.resolve("./node_builtins/os.cjs"),
			path: path.resolve("./node_builtins/path.cjs"),
			stream: path.resolve("./node_builtins/stream.cjs"),
			url: path.resolve("./node_builtins/url.cjs"),
			util: path.resolve("./node_builtins/util.cjs"),
		}),
	],
	sourcemap: true,
});
