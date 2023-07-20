import { Artifact } from "./artifact.ts";
import { assert } from "./assert.ts";
import { Block } from "./block.ts";
import { Directory } from "./directory.ts";
import { Relpath, subpath } from "./path.ts";
import * as syscall from "./syscall.ts";

type Arg = {
	module: syscall.Module;
	path: Relpath.Arg;
};

export let include = async (arg: Arg): Promise<Artifact> => {
	assert(arg.module.kind === "normal");
	let artifact = await Artifact.get(
		Block.fromSyscall(arg.module.value.package),
	);
	Directory.assert(artifact);
	let path = subpath(arg.module.value.modulePath)
		.toRelpath()
		.parent()
		.join(arg.path)
		.toSubpath();
	let includedArtifact = artifact.get(path);
	return includedArtifact;
};
