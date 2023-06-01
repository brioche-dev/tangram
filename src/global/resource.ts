import { Artifact } from "./artifact.ts";
import { assert as assert_ } from "./assert.ts";
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
	hash: Operation.Hash;
	url: string;
	unpack?: boolean;
	checksum?: Checksum;
	unsafe?: boolean;
};

export class Resource {
	#hash: Operation.Hash;
	#url: string;
	#unpack: boolean;
	#checksum?: Checksum;
	#unsafe: boolean;

	static async new(arg: Resource.Arg): Promise<Resource> {
		return Resource.fromSyscall(
			syscall.resource.new({
				url: arg.url,
				unpack: arg.unpack ?? false,
				checksum: arg.checksum ?? undefined,
				unsafe: arg.unsafe ?? false,
			}),
		);
	}

	constructor(arg: ConstructorArg) {
		this.#hash = arg.hash;
		this.#url = arg.url;
		this.#unpack = arg.unpack ?? false;
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

	hash(): Operation.Hash {
		return this.#hash;
	}

	toSyscall(): syscall.Resource {
		return {
			hash: this.#hash,
			url: this.#url,
			unpack: this.#unpack,
			checksum: this.#checksum,
			unsafe: this.#unsafe,
		};
	}

	static fromSyscall(download: syscall.Resource): Resource {
		return new Resource({
			hash: download.hash,
			url: download.url,
			unpack: download.unpack,
			checksum: download.checksum,
			unsafe: download.unsafe,
		});
	}

	async download(): Promise<Artifact> {
		let outputFromSyscall = await syscall.operation.run(
			Operation.toSyscall(this),
		);
		let output = Value.fromSyscall(outputFromSyscall);
		return output as Artifact;
	}
}

export namespace Resource {
	export type Arg = {
		url: string;
		unpack?: boolean;
		checksum?: Checksum;
		unsafe?: boolean;
	};
}
