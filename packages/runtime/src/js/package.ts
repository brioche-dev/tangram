import { Artifact } from "./artifact.ts";
import { assert as assert_ } from "./assert.ts";
import { Object_ } from "./object.ts";
import * as syscall from "./syscall.ts";

export class Package {
	#state: Package.State;

	constructor(state: Package.State) {
		this.#state = state;
	}

	get state(): Package.State {
		return this.#state;
	}

	static withId(id: Package.Id): Package {
		return new Package({ id });
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
		await this.store();
		return this.#state.id!;
	}

	async object(): Promise<Package.Object_> {
		await this.load();
		return this.#state.object!;
	}

	async load() {
		if (this.#state.object === undefined) {
			let object = await syscall.load(this.#state.id!);
			assert_(object.kind === "package");
			this.#state.object = object.value;
		}
	}

	async store() {
		if (this.#state.id === undefined) {
			this.#state.id = await syscall.store({
				kind: "package",
				value: this.#state.object!,
			});
		}
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

	export type State = Object_.State<Package.Id, Package.Object_>;
}
