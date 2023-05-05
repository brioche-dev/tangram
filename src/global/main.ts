import { Artifact } from "./artifact.ts";
import { Blob, blob } from "./blob.ts";
import { call } from "./call.ts";
import { Directory, directory } from "./directory.ts";
import { download } from "./download.ts";
import { env } from "./env.ts";
import { prepareStackTrace } from "./error.ts";
import { File, file } from "./file.ts";
import { Function, function_ } from "./function.ts";
import { include } from "./include.ts";
import { log } from "./log.ts";
import { Path, path } from "./path.ts";
import { Placeholder, placeholder } from "./placeholder.ts";
import { output, process } from "./process.ts";
import { resolve } from "./resolve.ts";
import { Symlink, symlink } from "./symlink.ts";
import { base64, hex, json, toml, utf8, yaml } from "./syscall.ts";
import { System, system } from "./system.ts";
import { Template, t, template } from "./template.ts";
import { Value, nullish } from "./value.ts";

// Set `Error.prepareStackTrace`.
Object.defineProperties(Error, {
	prepareStackTrace: { value: prepareStackTrace },
});

// Create the console global.
let console = {
	log,
};
Object.defineProperties(globalThis, {
	console: { value: console },
});

// Create the tg global.
let tg = {
	Artifact,
	Blob,
	Directory,
	File,
	Function,
	Path,
	Placeholder,
	Symlink,
	System,
	Template,
	Value,
	base64,
	blob,
	call,
	directory,
	download,
	env,
	file,
	function: function_,
	hex,
	include,
	json,
	log,
	nullish,
	output,
	path,
	placeholder,
	process,
	resolve,
	symlink,
	system,
	template,
	toml,
	utf8,
	yaml,
};
Object.defineProperties(globalThis, {
	tg: { value: tg },
	t: { value: t },
});
