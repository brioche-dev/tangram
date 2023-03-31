import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Symlink } from "./symlink.ts";
import * as syscall from "./syscall.ts";

export type ArtifactHash = string;

export type ArtifactKind = "directory" | "file" | "symlink";

export type Artifact = Directory | File | Symlink;

export let isArtifact = (value: unknown): value is Artifact => {
	return (
		value instanceof Directory ||
		value instanceof File ||
		value instanceof Symlink
	);
};

export let addArtifact = async (artifact: Artifact): Promise<ArtifactHash> => {
	return await syscall.addArtifact(await serializeArtifact(artifact));
};

export let getArtifact = async (hash: ArtifactHash): Promise<Artifact> => {
	return await deserializeArtifact(await syscall.getArtifact(hash));
};

export let serializeArtifact = async (
	artifact: Artifact,
): Promise<syscall.Artifact> => {
	if (artifact instanceof Directory) {
		return {
			kind: "directory",
			value: await artifact.serialize(),
		};
	} else if (artifact instanceof File) {
		return {
			kind: "file",
			value: await artifact.serialize(),
		};
	} else if (artifact instanceof Symlink) {
		return {
			kind: "symlink",
			value: await artifact.serialize(),
		};
	} else {
		throw new Error("Unknown artifact type");
	}
};

export let deserializeArtifact = async (
	artifact: syscall.Artifact,
): Promise<Artifact> => {
	switch (artifact.kind) {
		case "directory": {
			return await Directory.deserialize(artifact.value);
		}
		case "file": {
			return await File.deserialize(artifact.value);
		}
		case "symlink": {
			return await Symlink.deserialize(artifact.value);
		}
	}
};
