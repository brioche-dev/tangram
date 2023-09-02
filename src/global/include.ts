import { Artifact } from "./artifact.ts";
import { Directory } from "./directory.ts";
import { Module } from "./module.ts";
import { Relpath, subpath } from "./path.ts";

type Arg = {
	module: Module;
	path: Relpath.Arg;
};

export let include = async (arg: Arg): Promise<Artifact> => {
	let artifact = await Artifact.withId(await arg.module.package);
	Directory.assert(artifact);
	let path = subpath(arg.module.path)
		.toRelpath()
		.parent()
		.join(arg.path)
		.toSubpath();
	let includedArtifact = await artifact.get(path);
	return includedArtifact;
};
