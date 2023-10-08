import * as esbuild from "esbuild";
import alias from "esbuild-plugin-alias";
import * as path from "path";

await esbuild.build({
	bundle: true,
	entryPoints: ["main.ts"],
	inject: ["node.js"],
	minify: true,
	outfile: "../../assets/lsp.js",
	plugins: [
		alias({
			assert: path.resolve("./node/assert.cjs"),
			crypto: path.resolve("./node/crypto.cjs"),
			events: path.resolve("./node/events.cjs"),
			fs: path.resolve("./node/fs.cjs"),
			module: path.resolve("./node/module.cjs"),
			os: path.resolve("./node/os.cjs"),
			path: path.resolve("./node/path.cjs"),
			stream: path.resolve("./node/stream.cjs"),
			url: path.resolve("./node/url.cjs"),
			util: path.resolve("./node/util.cjs"),
		}),
	],
	sourcemap: true,
});
