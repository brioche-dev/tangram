import { Artifact } from "./artifact.ts";
import * as syscall from "./syscall.ts";

export let include = async (path: string): Promise<Artifact> => {
	let caller = syscall.stackFrame(1);
	let artifact = Artifact.fromSyscall(await syscall.include(caller, path));
	return artifact;
};
