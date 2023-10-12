import { assert } from "./assert.ts";
import * as encoding from "./encoding.ts";
import { Module } from "./module.ts";
import { Target, functions } from "./target.ts";
import { Value } from "./value.ts";

export let main = async (target: Target): Promise<Value> => {
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
