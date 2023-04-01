import { getArtifact } from "./artifact.ts";
import { Directory, isDirectory } from "./directory.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { assert } from "./util.ts";

export let bundle = async (
	artifact: Unresolved<Directory>,
): Promise<Directory> => {
	artifact = await resolve(artifact);
	let hash = await artifact.hash();
	let bundledHash = await syscall.bundle(hash);
	let bundledArtifact = await getArtifact(bundledHash);
	assert(isDirectory(bundledArtifact));
	return bundledArtifact;
};
