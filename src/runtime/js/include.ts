import { Artifact } from "./artifact.ts";
import { assert } from "./assert.ts";
import { Directory } from "./directory.ts";
import { Module } from "./module.ts";
import { Package } from "./package.ts";
import { Relpath, subpath } from "./path.ts";

type Arg = {
	url: string;
	path: Relpath.Arg;
};

export let include = async (arg: Arg): Promise<Artifact> => {
	let module_ = Module.fromUrl(arg.url);
	assert(module_.kind === "normal");
	let package_ = Package.withId(module_.value.packageId);
	let artifact = await package_.artifact();
	Directory.assert(artifact);
	let path = subpath(module_.value.path)
		.toRelpath()
		.parent()
		.join(arg.path)
		.toSubpath()
		.toString();
	let includedArtifact = await artifact.get(path);
	return includedArtifact;
};
