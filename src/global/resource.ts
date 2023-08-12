import { Artifact } from "./artifact.ts";
import { assert as assert_ } from "./assert.ts";
import { Block } from "./block.ts";
import { Checksum } from "./checksum.ts";
import { Id } from "./id.ts";
import * as syscall from "./syscall.ts";

export let resource = async (arg: Resource.Arg): Promise<Resource> => {
	return await Resource.new(arg);
};

export let download = async (arg: Resource.Arg): Promise<Artifact> => {
	let resource = await Resource.new(arg);
	let output = await resource.download();
	return output;
};

type ConstructorArg = {
	block: Block;
	url: string;
	unpack?: Resource.UnpackFormat;
	checksum?: Checksum;
	unsafe?: boolean;
};

export class Resource {
	#block: Block;
	#url: string;
	#unpack?: Resource.UnpackFormat;
	#checksum?: Checksum;
	#unsafe: boolean;

	constructor(arg: ConstructorArg) {
		this.#block = arg.block;
		this.#url = arg.url;
		this.#unpack = arg.unpack ?? undefined;
		this.#checksum = arg.checksum ?? undefined;
		this.#unsafe = arg.unsafe ?? false;
	}

	static async new(arg: Resource.Arg): Promise<Resource> {
		return await syscall.resource.new({
			url: arg.url,
			unpack: arg.unpack ?? undefined,
			checksum: arg.checksum ?? undefined,
			unsafe: arg.unsafe ?? false,
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

	id(): Id {
		return this.block().id();
	}

	block(): Block {
		return this.#block;
	}

	/** Get this resource's URL. */
	url(): string {
		return this.#url;
	}

	unpack(): Resource.UnpackFormat | undefined {
		return this.#unpack;
	}

	checksum(): Checksum | undefined {
		return this.#checksum;
	}

	unsafe(): boolean {
		return this.#unsafe;
	}

	async download(): Promise<Artifact> {
		return (await syscall.operation.evaluate(this)) as Artifact;
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
}
