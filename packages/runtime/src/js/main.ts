import { assert } from "./assert.ts";
import * as encoding from "./encoding.ts";
import { Module } from "./module.ts";
import { resolve } from "./resolve.ts";
import { Symlink } from "./symlink.ts";
import { Target, functions, setCurrent } from "./target.ts";
import { Value } from "./value.ts";

export let main = async (target: Target): Promise<Value> => {
	// Load the executable.
	let lock = await target.lock();
	assert(lock);
	let lockId = await lock.id();
	let executable = await target.executable();
	Symlink.assert(executable);
	let package_ = await executable.artifact();
	assert(package_);
	let packageId = await package_.id();
	let path = await executable.path();
	assert(path);
	let module_ = {
		kind: "normal" as const,
		value: { lock: lockId, package: packageId, path: path.toString() },
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

	// Set the current target.
	setCurrent(target);

	// Get the args.
	let args = await target.args();

	// Call the function.
	let output = await resolve(function_(...args));

	return output;
};
