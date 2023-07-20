import { assert as assert_, unreachable } from "./assert.ts";
import * as encoding from "./encoding.ts";
import { Id } from "./id.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { MaybeNestedArray, flatten } from "./util.ts";

export let block = async (...args: Array<Unresolved<Block.Arg>>) => {
	return await Block.new(...args);
};

type ConstructorArg = {
	id: Id;
};

export class Block {
	#id: Id;

	static async new(...args: Array<Unresolved<Block.Arg>>): Promise<Block> {
		// Collect the children and data from the args.
		let { children, data: dataEntries } = flatten(
			await Promise.all(
				args.map(async function map(unresolvedArg): Promise<
					MaybeNestedArray<{
						children?: Array<Block>;
						data?: Array<Uint8Array>;
					}>
				> {
					let arg = await resolve(unresolvedArg);
					if (arg === undefined) {
						return {};
					} else if (typeof arg === "string") {
						return { data: [encoding.utf8.encode(arg)] };
					} else if (arg instanceof Uint8Array) {
						return { data: [arg] };
					} else if (arg instanceof Block) {
						return {
							children: await arg.children(),
							data: [await arg.bytes()],
						};
					} else if (arg instanceof Array) {
						return await Promise.all(arg.map(map));
					} else if (typeof arg === "object") {
						let children = await Promise.all(
							(arg.children ?? []).map((child) => Block.new(child)),
						);
						let data =
							typeof arg.data === "string"
								? [encoding.utf8.encode(arg.data)]
								: arg.data instanceof Uint8Array
								? [arg.data]
								: [];
						return {
							children,
							data,
						};
					} else {
						return unreachable();
					}
				}),
			),
		).reduce<{
			children: Array<Block>;
			data: Array<Uint8Array>;
		}>(
			(value, { children, data }) => {
				if (children !== undefined) {
					value.children.push(...children);
				}
				if (data !== undefined) {
					value.data.push(...data);
				}
				return value;
			},
			{ children: [], data: [] },
		);

		// Collect the data entries.
		let length = dataEntries.reduce(
			(length, dataEntry) => length + dataEntry.length,
			0,
		);
		let data = new Uint8Array(length);
		let position = 0;
		for (let dataEntry of dataEntries) {
			data.set(dataEntry, position);
			position += dataEntry.length;
		}

		return Block.fromSyscall(
			await syscall.block.new({
				data,
				children: children.map((block) => block.toSyscall()),
			}),
		);
	}

	constructor(arg: ConstructorArg) {
		this.#id = arg.id;
	}

	static is(value: unknown): value is Block {
		return value instanceof Block;
	}

	static expect(value: unknown): Block {
		assert_(Block.is(value));
		return value;
	}

	static assert(value: unknown): asserts value is Block {
		assert_(Block.is(value));
	}

	toSyscall(): syscall.Block {
		return {
			id: this.#id,
		};
	}

	static fromSyscall(value: syscall.Block): Block {
		let id = value.id;
		return new Block({ id });
	}

	id(): Id {
		return this.#id;
	}

	async bytes(): Promise<Uint8Array> {
		return await syscall.block.bytes(this.toSyscall());
	}

	async children(): Promise<Array<Block>> {
		return (await syscall.block.children(this.toSyscall())).map((block) =>
			Block.fromSyscall(block),
		);
	}

	async data(): Promise<Uint8Array> {
		return await syscall.block.data(this.toSyscall());
	}
}

export namespace Block {
	export type Arg =
		| undefined
		| string
		| Uint8Array
		| Block
		| Array<Arg>
		| ArgObject;

	export type ArgObject = {
		children?: Array<Arg>;
		data?: string | Uint8Array;
	};

	export namespace Arg {
		export let is = (value: unknown): value is Arg => {
			return (
				value === undefined ||
				typeof value === "string" ||
				value instanceof Uint8Array ||
				value instanceof Block ||
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
