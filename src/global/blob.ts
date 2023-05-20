import { assert as assert_ } from "./assert.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";

export let blob = async (arg: Blob.Arg) => {
	return await Blob.new(arg);
};

type ConstructorArg = {
	hash: Blob.Hash;
};

export class Blob {
	#hash: Blob.Hash;

	static async new(arg: Unresolved<Blob.Arg>): Promise<Blob> {
		let resolvedArg = await resolve(arg);
		let bytes: Uint8Array | string;
		if (resolvedArg instanceof Uint8Array || typeof resolvedArg === "string") {
			bytes = resolvedArg;
		} else {
			return resolvedArg;
		}
		return Blob.fromSyscall(await syscall.blob.new(bytes));
	}

	constructor(arg: ConstructorArg) {
		this.#hash = arg.hash;
	}

	static is(value: unknown): value is Blob {
		return value instanceof Blob;
	}

	static expect(value: unknown): Blob {
		assert_(Blob.is(value));
		return value;
	}

	static assert(value: unknown): asserts value is Blob {
		assert_(Blob.is(value));
	}

	toSyscall(): syscall.Blob {
		return {
			hash: this.#hash,
		};
	}

	static fromSyscall(value: syscall.Blob): Blob {
		let hash = value.hash;
		return new Blob({ hash });
	}

	hash(): Blob.Hash {
		return this.#hash;
	}

	async bytes(): Promise<Uint8Array> {
		return await syscall.blob.bytes(this.toSyscall());
	}

	async text(): Promise<string> {
		return await syscall.blob.text(this.toSyscall());
	}
}

export namespace Blob {
	export type Arg = Uint8Array | string | Blob;

	export namespace Arg {
		export let is = (value: unknown): value is Blob.Arg => {
			return (
				value instanceof Uint8Array ||
				typeof value === "string" ||
				value instanceof Blob
			);
		};

		export let expect = (value: unknown): Arg => {
			assert_(is(value));
			return value;
		};

		export let assert = (value: unknown): asserts value is Arg => {
			assert_(is(value));
		};
	}

	export type Hash = string;
}
