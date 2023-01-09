import { Dependency } from "./dependency.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Symlink } from "./symlink.ts";

export class ArtifactHash {
	#string: string;

	constructor(string: string) {
		this.#string = string;
	}

	toString(): string {
		return this.#string;
	}
}

export type ArtifactType = "directory" | "file" | "symlink" | "dependency";

export type Artifact = Directory | File | Symlink | Dependency;

export let isArtifact = (value: unknown): value is Artifact => {
	return (
		value instanceof Directory ||
		value instanceof File ||
		value instanceof Symlink ||
		value instanceof Dependency
	);
};

export let addArtifact = async (artifact: Artifact): Promise<ArtifactHash> => {
	return new ArtifactHash(
		await syscall("add_artifact", await serializeArtifact(artifact)),
	);
};

export let getArtifact = async (hash: ArtifactHash): Promise<Artifact> => {
	return await deserializeArtifact(
		await syscall("get_artifact", hash.toString()),
	);
};

export let serializeArtifact = async (
	artifact: Artifact,
): Promise<syscall.Artifact> => {
	if (artifact instanceof Directory) {
		return {
			type: "directory",
			value: await artifact.serialize(),
		};
	} else if (artifact instanceof File) {
		return {
			type: "file",
			value: await artifact.serialize(),
		};
	} else if (artifact instanceof Symlink) {
		return {
			type: "symlink",
			value: await artifact.serialize(),
		};
	} else if (artifact instanceof Dependency) {
		return {
			type: "dependency",
			value: await artifact.serialize(),
		};
	} else {
		throw new Error("Unknown artifact type");
	}
};

export let deserializeArtifact = async (
	artifact: syscall.Artifact,
): Promise<Artifact> => {
	switch (artifact.type) {
		case "directory": {
			return await Directory.deserialize(artifact.value);
		}
		case "file": {
			return await File.deserialize(artifact.value);
		}
		case "symlink": {
			return await Symlink.deserialize(artifact.value);
		}
		case "dependency": {
			return await Dependency.deserialize(artifact.value);
		}
	}
};
