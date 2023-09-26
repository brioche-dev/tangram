import { Artifact } from "./artifact.ts";
import { Directory } from "./directory.ts";
import { Module } from "./module.ts";
import { Relpath, subpath } from "./path.ts";

type Arg = {
	module: Module;
	path: Relpath.Arg;
};

export let include = async (arg: Arg): Promise<Artifact> => {
	let artifact = await arg.module.package.artifact();
	Directory.assert(artifact);
	let path = subpath(arg.module.path)
		.toRelpath()
		.parent()
		.join(arg.path)
		.toSubpath()
		.toString();
	let includedArtifact = await artifact.get(path);
	return includedArtifact;
};
