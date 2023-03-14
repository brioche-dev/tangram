import {
	Artifact,
	ArtifactHash,
	addArtifact,
	getArtifact,
	getArtifactHash,
} from "./artifact";
import { Path, PathLike, path } from "./path";
import { MaybePromise } from "./resolve";
import { assert } from "./util";
import { isNullish, nullish } from "./value";

type ReferenceArgs = {
	artifact: MaybePromise<Artifact>;
	path?: PathLike | nullish;
};

export let reference = async (args: ReferenceArgs): Promise<Reference> => {
	let artifactHash = await addArtifact(await args.artifact);
	let path_ = !isNullish(args.path) ? path(args.path) : args.path;
	return new Reference({
		artifactHash,
		path: path_,
	});
};

export let isReference = (value: unknown): value is Reference => {
	return value instanceof Reference;
};

type ReferenceConstructorArgs = {
	artifactHash: ArtifactHash;
	path?: Path | nullish;
};

export class Reference {
	#artifactHash: ArtifactHash;
	#path: Path | nullish;

	constructor(args: ReferenceConstructorArgs) {
		this.#artifactHash = args.artifactHash;
		this.#path = args.path;
	}

	static async fromHash(hash: ArtifactHash): Promise<Reference> {
		let artifact = await getArtifact(hash);
		assert(isReference(artifact));
		return artifact;
	}

	async serialize(): Promise<syscall.Reference> {
		let artifactHash = this.#artifactHash;
		let path = this.#path?.toString();
		return {
			artifactHash,
			path,
		};
	}

	static async deserialize(reference: syscall.Reference): Promise<Reference> {
		let artifactHash = reference.artifactHash;
		let path_ = !isNullish(reference.path)
			? path(reference.path)
			: reference.path;
		return new Reference({
			artifactHash,
			path: path_,
		});
	}

	hash(): Promise<ArtifactHash> {
		return getArtifactHash(this);
	}

	artifactHash(): ArtifactHash {
		return this.#artifactHash;
	}

	path(): Path | nullish {
		return this.#path;
	}

	async getArtifact(): Promise<Artifact> {
		let artifact = await getArtifact(this.#artifactHash);
		return artifact;
	}
}
