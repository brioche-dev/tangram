import * as esbuild from "esbuild";
import alias from "esbuild-plugin-alias";
import * as path from "path";

await esbuild.build({
	bundle: true,
	entryPoints: ["./src/lsp.ts"],
	inject: ["./src/node.js"],
	minify: true,
	outfile: "./src/lsp.js",
	plugins: [
		alias({
			assert: path.resolve("./src/node/assert.cjs"),
			crypto: path.resolve("./src/node/crypto.cjs"),
			events: path.resolve("./src/node/events.cjs"),
			fs: path.resolve("./src/node/fs.cjs"),
			module: path.resolve("./src/node/module.cjs"),
			os: path.resolve("./src/node/os.cjs"),
			path: path.resolve("./src/node/path.cjs"),
			stream: path.resolve("./src/node/stream.cjs"),
			url: path.resolve("./src/node/url.cjs"),
			util: path.resolve("./src/node/util.cjs"),
		}),
	],
	sourcemap: true,
});
