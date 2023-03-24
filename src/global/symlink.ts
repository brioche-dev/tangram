import { ArtifactHash, addArtifact, getArtifact } from "./artifact";
import * as syscall from "./syscall";
import { assert } from "./util";

export let symlink = (target: string): Symlink => {
	return new Symlink(target);
};

export let isSymlink = (value: unknown): value is Symlink => {
	return value instanceof Symlink;
};

export class Symlink {
	#target: string;

	constructor(target: string) {
		this.#target = target;
	}

	static async fromHash(hash: ArtifactHash): Promise<Symlink> {
		let artifact = await getArtifact(hash);
		assert(isSymlink(artifact));
		return artifact;
	}

	async serialize(): Promise<syscall.Symlink> {
		return {
			target: this.#target,
		};
	}

	static async deserialize(symlink: syscall.Symlink): Promise<Symlink> {
		return new Symlink(symlink.target);
	}

	hash(): Promise<ArtifactHash> {
		return addArtifact(this);
	}

	target(): string {
		return this.#target;
	}
}
