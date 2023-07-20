import { Artifact } from "./artifact.ts";
import { assert as assert_ } from "./assert.ts";
import { Block } from "./block.ts";
import { Checksum } from "./checksum.ts";
import { Operation } from "./operation.ts";
import * as syscall from "./syscall.ts";
import { Value } from "./value.ts";

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

	static async new(arg: Resource.Arg): Promise<Resource> {
		return Resource.fromSyscall(
			syscall.resource.new({
				url: arg.url,
				unpack: arg.unpack ?? undefined,
				checksum: arg.checksum ?? undefined,
				unsafe: arg.unsafe ?? false,
			}),
		);
	}

	constructor(arg: ConstructorArg) {
		this.#block = arg.block;
		this.#url = arg.url;
		this.#unpack = arg.unpack ?? undefined;
		this.#checksum = arg.checksum ?? undefined;
		this.#unsafe = arg.unsafe ?? false;
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

	block(): Block {
		return this.#block;
	}

	toSyscall(): syscall.Resource {
		return {
			block: this.#block.toSyscall(),
			url: this.#url,
			unpack: this.#unpack,
			checksum: this.#checksum,
			unsafe: this.#unsafe,
		};
	}

	static fromSyscall(download: syscall.Resource): Resource {
		return new Resource({
			block: Block.fromSyscall(download.block),
			url: download.url,
			unpack: download.unpack,
			checksum: download.checksum,
			unsafe: download.unsafe,
		});
	}

	async download(): Promise<Artifact> {
		let outputFromSyscall = await syscall.operation.evaluation(
			Operation.toSyscall(this),
		);
		let output = Value.fromSyscall(outputFromSyscall);
		return output as Artifact;
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
