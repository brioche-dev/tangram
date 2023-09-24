import { Artifact } from "./artifact.ts";
import { assert as assert_ } from "./assert.ts";
import { Object_ } from "./object.ts";

export class Package {
	#handle: Object_.Handle;

	constructor(handle: Object_.Handle) {
		this.#handle = handle;
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

	async artifact(): Promise<Artifact> {
		let object = (await this.#handle.object()) as Package.Object;
		return object.artifact;
	}

	async dependencies(): Promise<{ [dependency: string]: Package }> {
		let object = (await this.#handle.object()) as Package.Object;
		return object.dependencies;
	}
}

export namespace Package {
	export type Object = {
		artifact: Artifact;
		dependencies: { [dependency: string]: Package };
	};
}
