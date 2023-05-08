import { Artifact } from "./artifact.ts";
import { Checksum } from "./checksum.ts";
import { Operation } from "./operation.ts";
import * as syscall from "./syscall.ts";
import { Value } from "./value.ts";

export let download = async (arg: Download.Arg): Promise<Artifact> => {
	// Create the download.
	let download = await Download.new(arg);

	// Run the operation.
	let output = await download.run();

	return output;
};

type ConstructorArg = {
	hash: Operation.Hash;
	url: string;
	unpack?: boolean;
	checksum?: Checksum;
	unsafe?: boolean;
};

export class Download {
	#hash: Operation.Hash;
	#url: string;
	#unpack: boolean;
	#checksum?: Checksum;
	#unsafe: boolean;

	static async new(arg: Download.Arg): Promise<Download> {
		return Download.fromSyscall(
			await syscall.download.new({
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

	static is(value: unknown): value is Download {
		return value instanceof Download;
	}

	hash(): Operation.Hash {
		return this.#hash;
	}

	toSyscall(): syscall.Download {
		return {
			hash: this.#hash,
			url: this.#url,
			unpack: this.#unpack,
			checksum: this.#checksum,
			unsafe: this.#unsafe,
		};
	}

	static fromSyscall(download: syscall.Download): Download {
		return new Download({
			hash: download.hash,
			url: download.url,
			unpack: download.unpack,
			checksum: download.checksum,
			unsafe: download.unsafe,
		});
	}

	async run(): Promise<Artifact> {
		let outputFromSyscall = await syscall.operation.run(
			Operation.toSyscall(this),
		);
		let output = Value.fromSyscall(outputFromSyscall);
		return output as Artifact;
	}
}

export namespace Download {
	export type Arg = {
		url: string;
		unpack?: boolean;
		checksum?: Checksum;
		unsafe?: boolean;
	};
}
