import { Artifact } from "./artifact.ts";
import { assert as assert_ } from "./assert.ts";
import { Object_ } from "./object.ts";

export class Package {
	#handle: Object_.Handle;

	constructor(handle: Object_.Handle) {
		this.#handle = handle;
	}

	static withId(id: Package.Id): Package {
		return new Package(Object_.Handle.withId(id));
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

	async id(): Promise<Package.Id> {
		return (await this.#handle.id()) as Package.Id;
	}

	async object(): Promise<Package.Object_> {
		let object = await this.#handle.object();
		assert_(object.kind === "package");
		return object.value;
	}

	get handle(): Object_.Handle {
		return this.#handle;
	}

	async artifact(): Promise<Artifact> {
		return (await this.object()).artifact;
	}

	async dependencies(): Promise<{ [dependency: string]: Package }> {
		return (await this.object()).dependencies;
	}
}

export namespace Package {
	export type Arg = Package | Array<Arg> | ArgObject;

	export type ArgObject = {
		artifact: Artifact;
		dependencies?: { [dependency: string]: Package.Arg };
	};

	export type Id = string;

	export type Object_ = {
		artifact: Artifact;
		dependencies: { [dependency: string]: Package };
	};
}
