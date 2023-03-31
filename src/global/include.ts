import { Artifact, deserializeArtifact, getArtifact } from "./artifact.ts";
import * as syscall from "./syscall.ts";

export let include = async (path: string): Promise<Artifact> => {
	let caller = syscall.caller();
	return await deserializeArtifact(await syscall.include(caller, path));
};
