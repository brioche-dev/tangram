import * as esbuild from "esbuild";
import alias from "esbuild-plugin-alias";
import * as path from "path";

// Run esbuild.
await esbuild.build({
	bundle: true,
	entryPoints: ["main.ts"],
	outfile: "../../../assets/language_service.js",
	minify: true,
	inject: ["node_global.js"],
	plugins: [
		alias({
			assert: path.resolve("./node_builtins/assert.cjs"),
			crypto: path.resolve("./node_builtins/crypto.js"),
			events: path.resolve("./node_builtins/events.js"),
			fs: path.resolve("./node_builtins/fs.js"),
			module: path.resolve("./node_builtins/module.js"),
			os: path.resolve("./node_builtins/os.js"),
			path: path.resolve("./node_builtins/path.js"),
			stream: path.resolve("./node_builtins/stream.js"),
			url: path.resolve("./node_builtins/url.js"),
			util: path.resolve("./node_builtins/util.js"),
		}),
	],
});
