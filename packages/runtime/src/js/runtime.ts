import { Artifact } from "./artifact.ts";
import { assert, unimplemented, unreachable } from "./assert.ts";
import { Blob, blob, download } from "./blob.ts";
import { Directory, directory } from "./directory.ts";
import * as encoding from "./encoding.ts";
import { Error as Error_, prepareStackTrace } from "./error.ts";
import { File, file } from "./file.ts";
import { include } from "./include.ts";
import { log } from "./log.ts";
import { main } from "./main.ts";
import { Object_ } from "./object.ts";
import { Package } from "./package.ts";
import { Placeholder, placeholder } from "./placeholder.ts";
import { resolve } from "./resolve.ts";
import { Symlink, symlink } from "./symlink.ts";
import { System, system } from "./system.ts";
import { Target, build, output, target } from "./target.ts";
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
	Error: Error_,
	File,
	Object_,
	Package,
	Placeholder,
	Symlink,
	System,
	Target,
	Template,
	Value,
	assert,
	blob,
	build,
	directory,
	download,
	encoding,
	file,
	include,
	log,
	main,
	output,
	placeholder,
	resolve,
	symlink,
	system,
	target,
	template,
	unimplemented,
	unreachable,
};
Object.defineProperties(globalThis, {
	tg: { value: tg },
	t: { value: t },
});
