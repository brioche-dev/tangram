import { Artifact } from "./artifact.ts";
import { Blob, blob } from "./blob.ts";
import { command, output, run } from "./command.ts";
import { Directory, directory } from "./directory.ts";
import { env } from "./env.ts";
import { prepareStackTrace } from "./error.ts";
import { File, file } from "./file.ts";
import { Function, entrypoint, function_, registry } from "./function.ts";
import { include } from "./include.ts";
import { log } from "./log.ts";
import { Relpath, Subpath, relpath, subpath } from "./path.ts";
import { Placeholder, placeholder } from "./placeholder.ts";
import { resolve } from "./resolve.ts";
import { download, resource } from "./resource.ts";
import { Symlink, symlink } from "./symlink.ts";
import { base64, hex, json, toml, utf8, yaml } from "./syscall.ts";
import { System, system } from "./system.ts";
import { Template, t, template } from "./template.ts";
import { Value } from "./value.ts";

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
	Placeholder,
	Relpath,
	Subpath,
	Symlink,
	System,
	Template,
	Value,
	base64,
	blob,
	command,
	directory,
	download,
	entrypoint,
	env,
	file,
	function: function_,
	hex,
	include,
	json,
	log,
	output,
	placeholder,
	registry,
	relpath,
	resolve,
	resource,
	run,
	subpath,
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
