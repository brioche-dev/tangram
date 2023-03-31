import { addArtifact, getArtifact } from "./artifact.ts";
import { Directory, isDirectory } from "./directory.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";

export let bundle = async (
	artifact: Unresolved<Directory>,
): Promise<Directory> => {
	let resolvedArtifact = await resolve(artifact);
	let hash = await addArtifact(resolvedArtifact);
	let vendoredHash = await syscall.bundle(hash);
	let vendoredArtifact = await getArtifact(vendoredHash);

	if (!isDirectory(vendoredArtifact)) {
		throw new Error("vendor syscall returned non-directory artifact.");
	}

	return vendoredArtifact;
};
