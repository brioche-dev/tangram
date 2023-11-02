import { Artifact } from "./artifact.ts";
import { assert } from "./assert.ts";
import { Directory } from "./directory.ts";
import { Module } from "./module.ts";
import { Relpath, subpath } from "./path.ts";

type Arg = {
	url: string;
	path: Relpath.Arg;
};

export let include = async (arg: Arg): Promise<Artifact> => {
	let module_ = Module.fromUrl(arg.url);
	assert(module_.kind === "normal");
	let package_ = Directory.withId(module_.value.package);
	let path = subpath(module_.value.path)
		.toRelpath()
		.parent()
		.join(arg.path)
		.toSubpath()
		.toString();
	let includedArtifact = await package_.get(path);
	return includedArtifact;
};
