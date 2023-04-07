import { Artifact } from "./artifact.ts";
import { Checksum } from "./checksum.ts";
import { Operation } from "./operation.ts";
import * as syscall from "./syscall.ts";
import { Value, nullish } from "./value.ts";

export namespace Download {
	export type Arg = {
		url: string;
		unpack?: boolean | nullish;
		checksum?: Checksum | nullish;
		unsafe?: boolean | nullish;
	};

	export type ConstructorArg = {
		hash: Operation.Hash;
		url: string;
		unpack?: boolean | nullish;
		checksum?: Checksum | nullish;
		unsafe?: boolean | nullish;
	};
}

export let download = async (arg: Download.Arg): Promise<Artifact> => {
	// Create the download.
	let download = Download.fromSyscall(
		await syscall.download.new(
			arg.url,
			arg.unpack ?? false,
			arg.checksum ?? null,
			arg.unsafe ?? false,
		),
	);

	// Run the operation.
	let output = await download.run();

	return output;
};

export class Download {
	#hash: Operation.Hash;
	#url: string;
	#unpack: boolean;
	#checksum: Checksum | nullish;
	#unsafe: boolean;

	constructor(arg: Download.ConstructorArg) {
		this.#hash = arg.hash;
		this.#url = arg.url;
		this.#unpack = arg.unpack ?? false;
		this.#checksum = arg.checksum ?? null;
		this.#unsafe = arg.unsafe ?? false;
	}

	static isDownload(value: unknown): value is Download {
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
