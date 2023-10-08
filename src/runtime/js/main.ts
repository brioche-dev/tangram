import { Artifact } from "./artifact.ts";
import { assert, unimplemented, unreachable } from "./assert.ts";
import { Blob, blob, download } from "./blob.ts";
import { Directory, directory } from "./directory.ts";
import * as encoding from "./encoding.ts";
import { Error as Error_, prepareStackTrace } from "./error.ts";
import { File, file } from "./file.ts";
import { include } from "./include.ts";
import { log } from "./log.ts";
import { Module } from "./module.ts";
import { Object_ } from "./object.ts";
import { Package } from "./package.ts";
import { Placeholder, placeholder } from "./placeholder.ts";
import { resolve } from "./resolve.ts";
import { Symlink, symlink } from "./symlink.ts";
import { System, system } from "./system.ts";
import { Target, build, functions, output, target } from "./target.ts";
import { Template, t, template } from "./template.ts";
import { Value } from "./value.ts";

let main = async (target: Target): Promise<Value> => {
	// Load the executable.
	let package_ = await target.package();
	assert(package_);
	let packageId = await package_.id();
	let executable = await target.executable();
	let path = executable.components[0];
	assert(typeof path === "string");
	let module_ = {
		kind: "normal" as const,
		value: { packageId, path },
	};
	let url = Module.toUrl(module_);
	await import(url);

	// Get the target.
	let name = await target.name_();
	if (!name) {
		throw new Error("The target must have a name.");
	}

	// Get the function.
	let key = encoding.json.encode({ url, name });
	let function_ = functions[key];
	if (!function_) {
		throw new Error("Failed to find the function.");
	}

	// Get the args.
	let args = await target.args();

	// Call the function.
	let output = await function_(...args);

	return output;
};

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
