import { assert as assert_, unreachable } from "./assert.ts";
import { Block, block } from "./block.ts";
import { Id } from "./id.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { MaybeNestedArray, flatten } from "./util.ts";

export let blob = async (...args: Array<Unresolved<Blob.Arg>>) => {
	return await Blob.new(...args);
};

type ConstructorArg = {
	block: Block;
	kind: unknown;
};

export class Blob {
	#block: Block;
	// @ts-ignore
	#kind: unknown;

	constructor(arg: ConstructorArg) {
		this.#block = arg.block;
		this.#kind = arg.kind;
	}

	static async withBlock(block: Block): Promise<Blob> {
		return await syscall.blob.get(block);
	}

	static async new(...args: Array<Unresolved<Blob.Arg>>): Promise<Blob> {
		let children = flatten(
			await Promise.all(
				args.map(async function map(
					unresolvedArg: Unresolved<Blob.Arg>,
				): Promise<MaybeNestedArray<Block>> {
					let arg = await resolve(unresolvedArg);
					if (Block.Arg.is(arg)) {
						return await block(arg);
					} else if (arg instanceof Blob) {
						return arg.block();
					} else if (arg instanceof Array) {
						return await Promise.all(arg.map(map));
					} else {
						return unreachable();
					}
				}),
			),
		).reduce<Array<Block>>((blocks, block) => {
			blocks.push(block);
			return blocks;
		}, []);
		return await syscall.blob.new({
			children,
		});
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

	id(): Id {
		return this.block().id();
	}

	block(): Block {
		return this.#block;
	}

	async bytes(): Promise<Uint8Array> {
		return await syscall.blob.bytes(this);
	}

	async text(): Promise<string> {
		return await syscall.blob.text(this);
	}
}

export namespace Blob {
	export type Arg = Block.Arg | Blob | Array<Arg>;

	export namespace Arg {
		export let is = (value: unknown): value is Arg => {
			return (
				Block.Arg.is(value) ||
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
	}
}
