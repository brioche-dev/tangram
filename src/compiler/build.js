import alias from "esbuild-plugin-alias";
import * as esbuild from "esbuild";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

// Get the path to the compiler directory.
let compilerDirectoryPath = dirname(fileURLToPath(import.meta.url));

// Run esbuild.
await esbuild.build({
	bundle: true,
	entryPoints: ["mod.ts"],
	globalName: "compiler",
	minify: true,
	inject: ["./node_global.js"],
	outfile: "mod.js",
	plugins: [
		alias({
			crypto: resolve(compilerDirectoryPath, `node_builtins/crypto.js`),
			events: resolve(compilerDirectoryPath, `node_builtins/events.js`),
			fs: resolve(compilerDirectoryPath, `node_builtins/fs.js`),
			module: resolve(compilerDirectoryPath, `node_builtins/module.js`),
			os: resolve(compilerDirectoryPath, `node_builtins/os.js`),
			path: resolve(compilerDirectoryPath, `node_builtins/path.js`),
			process: resolve(compilerDirectoryPath, `node_builtins/process.js`),
			stream: resolve(compilerDirectoryPath, `node_builtins/stream.js`),
			url: resolve(compilerDirectoryPath, `node_builtins/url.js`),
			util: resolve(compilerDirectoryPath, `node_builtins/util.js`),
		}),
	],
});
