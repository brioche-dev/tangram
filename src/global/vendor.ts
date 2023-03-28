import { addArtifact, getArtifact } from "./artifact";
import { Directory, isDirectory } from "./directory";
import { Unresolved, resolve } from "./resolve";
import * as syscall from "./syscall";

export let vendor = async (
	artifact: Unresolved<Directory>,
): Promise<Directory> => {
	let resolvedArtifact = await resolve(artifact);
	let hash = await addArtifact(resolvedArtifact);
	let vendoredHash = await syscall.vendor(hash);
	let vendoredArtifact = await getArtifact(vendoredHash);

	if (!isDirectory(vendoredArtifact)) {
		throw new Error("vendor syscall returned non-directory artifact.");
	}

	return vendoredArtifact;
};
