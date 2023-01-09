import { Artifact } from "./artifact.ts";

export let symlink = (target: string): Symlink => {
	return new Symlink(target);
};

export class Symlink {
	target: string;

	constructor(target: string) {
		this.target = target;
	}

	async serialize(): Promise<syscall.Symlink> {
		return {
			target: this.target,
		};
	}

	static async deserialize(symlink: syscall.Symlink): Promise<Symlink> {
		return new Symlink(symlink.target);
	}
}
