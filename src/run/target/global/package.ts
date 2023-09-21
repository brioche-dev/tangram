import { Artifact } from "./artifact.ts";
import { assert as assert_ } from "./assert.ts";
import { Id } from "./id.ts";
import * as syscall from "./syscall.ts";

export class Package {
	#id: Id | undefined;
	#data: Package.Data | undefined;

	constructor(arg: Package.Data) {
		this.#data = arg;
	}

	static is(value: unknown): value is Package {
		return value instanceof Package;
	}

	static expect(value: unknown): Package {
		assert_(Package.is(value));
		return value;
	}

	static assert(value: unknown): asserts value is Package {
		assert_(Package.is(value));
	}

	async load(): Promise<void> {
		if (!this.#data) {
			this.#data = ((await syscall.value.load(this)) as Package).#data;
		}
	}

	async store(): Promise<void> {
		if (!this.#id) {
			this.#id = ((await syscall.value.store(this)) as Package).#id;
		}
	}

	async artifact(): Promise<Artifact> {
		await this.load();
		return this.#data!.artifact;
	}

	async dependencies(): Promise<{ [dependency: string]: Package }> {
		await this.load();
		return this.#data!.dependencies;
	}
}

export namespace Package {
	export type Data = {
		artifact: Artifact;
		dependencies: { [dependency: string]: Package };
	};
}
