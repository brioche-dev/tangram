import { Directory } from "./directory";
import { File } from "./file";
import { Reference as Reference } from "./reference";
import { Symlink } from "./symlink";

export type ArtifactHash = string;

export type ArtifactKind = "directory" | "file" | "symlink" | "reference";

export type Artifact = Directory | File | Symlink | Reference;

export let isArtifact = (value: unknown): value is Artifact => {
	return (
		value instanceof Directory ||
		value instanceof File ||
		value instanceof Symlink ||
		value instanceof Reference
	);
};

export let addArtifact = async (artifact: Artifact): Promise<ArtifactHash> => {
	return await syscall("add_artifact", await serializeArtifact(artifact));
};

export let getArtifact = async (hash: ArtifactHash): Promise<Artifact> => {
	return await deserializeArtifact(await syscall("get_artifact", hash));
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
	} else if (artifact instanceof Reference) {
		return {
			kind: "reference",
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
		case "reference": {
			return await Reference.deserialize(artifact.value);
		}
	}
};
