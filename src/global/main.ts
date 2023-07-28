import { Artifact } from "./artifact.ts";
import { assert, unimplemented, unreachable } from "./assert.ts";
import { Blob, blob } from "./blob.ts";
import { Block } from "./block.ts";
import { Directory, directory } from "./directory.ts";
import * as encoding from "./encoding.ts";
import { env } from "./env.ts";
import { prepareStackTrace } from "./error.ts";
import { File, file } from "./file.ts";
import { include } from "./include.ts";
import { log } from "./log.ts";
import { Relpath, Subpath, relpath, subpath } from "./path.ts";
import { Placeholder, placeholder } from "./placeholder.ts";
import { resolve } from "./resolve.ts";
import { download, resource } from "./resource.ts";
import { Symlink, symlink } from "./symlink.ts";
import { System, system } from "./system.ts";
import { Target, entrypoint, target, targets } from "./target.ts";
import { Task, output, run, task } from "./task.ts";
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
	Block,
	Directory,
	File,
	Placeholder,
	Relpath,
	Subpath,
	Symlink,
	System,
	Target,
	Task,
	Template,
	Value,
	assert,
	blob,
	directory,
	download,
	encoding,
	entrypoint,
	env,
	file,
	include,
	log,
	output,
	placeholder,
	relpath,
	resolve,
	resource,
	run,
	subpath,
	symlink,
	system,
	target,
	targets,
	task,
	template,
	unimplemented,
	unreachable,
};
Object.defineProperties(globalThis, {
	tg: { value: tg },
	t: { value: t },
});
