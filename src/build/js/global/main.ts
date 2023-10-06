import { Artifact } from "./artifact.ts";
import { assert, unimplemented, unreachable } from "./assert.ts";
import { Blob, blob } from "./blob.ts";
import { Directory, directory } from "./directory.ts";
import { download, unpack } from "./download.ts";
import * as encoding from "./encoding.ts";
import { Error as Error_, prepareStackTrace } from "./error.ts";
import { File, file } from "./file.ts";
import { include } from "./include.ts";
import { log } from "./log.ts";
import { Object_ } from "./object.ts";
import { Package } from "./package.ts";
import { Relpath, Subpath, relpath, subpath } from "./path.ts";
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
	let packageId = await package_?.id();
	let executable = await target.executable();
	let path = executable.components()[0];
	let module_ = { kind: "normal", value: { package: packageId, path } };
	let data = encoding.hex.encode(
		encoding.utf8.encode(encoding.json.encode(module_)),
	);
	let url = `tangram://${data}/${path}`;
	await import(url);

	// Get the target.
	let name = await target.name_();
	if (!name) {
		throw new Error("The target must have a name.");
	}

	// Get the function.
	let key = encoding.json.encode({
		module: { package: packageId, path },
		name,
	});
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
	Relpath,
	Subpath,
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
	relpath,
	resolve,
	subpath,
	symlink,
	system,
	target,
	template,
	unimplemented,
	unpack,
	unreachable,
};
Object.defineProperties(globalThis, {
	tg: { value: tg },
	t: { value: t },
});
