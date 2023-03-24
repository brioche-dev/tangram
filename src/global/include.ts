import { Artifact, deserializeArtifact, getArtifact } from "./artifact";
import * as syscall from "./syscall";

export let include = async (path: string): Promise<Artifact> => {
	let caller = syscall.caller();
	return await deserializeArtifact(await syscall.include(caller, path));
};
