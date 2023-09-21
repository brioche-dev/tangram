import { assert as assert_, unreachable } from "./assert.ts";
import * as encoding from "./encoding.ts";
import { Ref as Ref_ } from "./ref.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { MaybeNestedArray, flatten } from "./util.ts";

export let blob = async (...args: Array<Unresolved<Blob.Arg>>) => {
	return await Blob.new(...args);
};

export class Blob {
	#kind: Blob.Kind;

	constructor(arg: Blob.Fields) {
		this.#kind = arg.kind;
	}

	static async new(...args: Array<Unresolved<Blob.Arg>>): Promise<Blob> {
		let children = flatten(
			await Promise.all(
				args.map(async function map(
					unresolvedArg: Unresolved<Blob.Arg>,
				): Promise<MaybeNestedArray<Blob>> {
					let arg = await resolve(unresolvedArg);
					if (arg === undefined) {
						return [];
					} else if (typeof arg === "string") {
						return new Blob({
							kind: {
								kind: "leaf",
								value: { bytes: encoding.utf8.encode(arg) },
							},
						});
					} else if (arg instanceof Uint8Array) {
						return new Blob({
							kind: {
								kind: "leaf",
								value: { bytes: arg },
							},
						});
					} else if (arg instanceof Blob) {
						return arg;
					} else if (arg instanceof Array) {
						return await Promise.all(arg.map(map));
					} else {
						return unreachable();
					}
				}),
			),
		);
		if (children.length === 0) {
			return new Blob({
				kind: { kind: "leaf", value: { bytes: new Uint8Array() } },
			});
		} else if (children.length === 1) {
			return children[0]!;
		} else {
			let childrenWithSizes: Array<[Blob, number]> = await Promise.all(
				children.map(async (child) => [child, await child.size()]),
			);
			return new Blob({
				kind: { kind: "branch", value: { children: childrenWithSizes } },
			});
		}
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

	/** Get this blob's size. */
	size(): number {
		switch (this.#kind!.kind) {
			case "branch": {
				let size = 0;
				for (let [_, childSize] of this.#kind.value.children) {
					size += childSize;
				}
				return size;
			}
			case "leaf": {
				return this.#kind.value.bytes.length;
			}
		}
	}

	async bytes(): Promise<Uint8Array> {
		return await syscall.blob.bytes(this);
	}

	async text(): Promise<string> {
		return await syscall.blob.text(this);
	}
}

export namespace Blob {
	export type Arg = undefined | string | Uint8Array | Blob | Array<Arg>;

	export namespace Arg {
		export let is = (value: unknown): value is Arg => {
			return (
				value === undefined ||
				typeof value === "string" ||
				value instanceof Uint8Array ||
				value instanceof Blob ||
				(value instanceof Array && value.every(Arg.is))
			);
		};

		export let expect = (value: unknown): Arg => {
			assert_(is(value));
			return value;
		};

		export let assert = (value: unknown): asserts value is Arg => {
			assert_(is(value));
		};

		export class Ref extends Ref_<Blob> {}
	}

	export type Fields = {
		kind: Kind;
	};

	export type Kind =
		| { kind: "branch"; value: { children: Array<[Blob, number]> } }
		| { kind: "leaf"; value: { bytes: Uint8Array } };
}
