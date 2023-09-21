import { Artifact } from "./artifact.ts";
import { assert as assert_ } from "./assert.ts";
import { Checksum } from "./checksum.ts";
import { Id } from "./id.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";

export let resource = async (
	arg: Unresolved<Resource.Arg>,
): Promise<Resource> => {
	return await Resource.new(arg);
};

export let download = async (
	arg: Unresolved<Resource.Arg>,
): Promise<Artifact> => {
	let resource = await Resource.new(arg);
	let output = await resource.download();
	return output;
};

export class Resource {
	#id: Id | undefined;
	#data: Resource.Data | undefined;

	constructor(arg: Resource.Data) {
		this.#data = arg;
	}

	static async new(arg: Unresolved<Resource.Arg>): Promise<Resource> {
		let resolvedArg = await resolve(arg);
		return new Resource({
			url: resolvedArg.url,
			unpack: resolvedArg.unpack ?? undefined,
			checksum: resolvedArg.checksum ?? undefined,
			unsafe: resolvedArg.unsafe ?? false,
		});
	}

	static is(value: unknown): value is Resource {
		return value instanceof Resource;
	}

	static expect(value: unknown): Resource {
		assert_(Resource.is(value));
		return value;
	}

	static assert(value: unknown): asserts value is Resource {
		assert_(Resource.is(value));
	}

	async load(): Promise<void> {
		if (!this.#data) {
			this.#data = ((await syscall.value.load(this)) as Resource).#data;
		}
	}

	async store(): Promise<void> {
		if (!this.#id) {
			this.#id = ((await syscall.value.store(this)) as Resource).#id;
		}
	}

	/** Get this resource's URL. */
	async url(): Promise<string> {
		await this.load();
		return this.#data!.url;
	}

	async unpack(): Promise<Resource.UnpackFormat | undefined> {
		await this.load();
		return this.#data!.unpack;
	}

	async checksum(): Promise<Checksum | undefined> {
		await this.load();
		return this.#data!.checksum;
	}

	async unsafe(): Promise<boolean> {
		await this.load();
		return this.#data!.unsafe;
	}

	async download(): Promise<Artifact> {
		return (await syscall.build.output(this)) as Artifact;
	}
}

export namespace Resource {
	export type Arg = {
		url: string;
		unpack?: UnpackFormat;
		checksum?: Checksum;
		unsafe?: boolean;
	};

	export type UnpackFormat =
		| ".tar"
		| ".tar.bz2"
		| ".tar.gz"
		| ".tar.lz"
		| ".tar.xz"
		| ".tar.zstd"
		| ".zip";

	export type Data = {
		url: string;
		unpack?: Resource.UnpackFormat;
		checksum?: Checksum;
		unsafe: boolean;
	};
}
