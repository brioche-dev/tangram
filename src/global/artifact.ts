import { unreachable } from "./assert.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Symlink } from "./symlink.ts";
import * as syscall from "./syscall.ts";

export type Artifact = Directory | File | Symlink;

export namespace Artifact {
	export type Hash = string;

	export let is = (value: unknown): value is Artifact => {
		return (
			value instanceof Directory ||
			value instanceof File ||
			value instanceof Symlink
		);
	};

	export let get = async (hash: Hash): Promise<Artifact> => {
		return Artifact.fromSyscall(await syscall.artifact.get(hash));
	};

	export let toSyscall = (artifact: Artifact): syscall.Artifact => {
		if (artifact instanceof Directory) {
			return {
				kind: "directory",
				value: artifact.toSyscall(),
			};
		} else if (artifact instanceof File) {
			return {
				kind: "file",
				value: artifact.toSyscall(),
			};
		} else if (artifact instanceof Symlink) {
			return {
				kind: "symlink",
				value: artifact.toSyscall(),
			};
		} else {
			return unreachable();
		}
	};

	export let fromSyscall = (artifact: syscall.Artifact): Artifact => {
		switch (artifact.kind) {
			case "directory": {
				return Directory.fromSyscall(artifact.value);
			}
			case "file": {
				return File.fromSyscall(artifact.value);
			}
			case "symlink": {
				return Symlink.fromSyscall(artifact.value);
			}
			default: {
				return unreachable();
			}
		}
	};
}
