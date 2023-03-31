import { ArtifactHash, addArtifact } from "./artifact.ts";
import { Unresolved } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { Template, TemplateLike, template } from "./template.ts";

export let symlink = async (
	target: Unresolved<TemplateLike>,
): Promise<Symlink> => {
	target = await template(target);
	return new Symlink(target);
};

export let isSymlink = (value: unknown): value is Symlink => {
	return value instanceof Symlink;
};

export class Symlink {
	#target: Template;

	constructor(target: Template) {
		this.#target = target;
	}

	async serialize(): Promise<syscall.Symlink> {
		let target = await this.#target.serialize();
		return {
			target,
		};
	}

	static async deserialize(symlink: syscall.Symlink): Promise<Symlink> {
		let target = await Template.deserialize(symlink.target);
		return new Symlink(target);
	}

	async hash(): Promise<ArtifactHash> {
		return await addArtifact(this);
	}

	target(): Template {
		return this.#target;
	}
}
