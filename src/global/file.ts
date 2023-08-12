import { Artifact } from "./artifact.ts";
import { assert as assert_, unreachable } from "./assert.ts";
import { Blob, blob } from "./blob.ts";
import { Block } from "./block.ts";
import { Id } from "./id.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { MaybeNestedArray, flatten } from "./util.ts";

export let file = async (...args: Array<Unresolved<File.Arg>>) => {
	return await File.new(...args);
};

type ConstructorArg = {
	block: Block;
	contents: Block;
	executable: boolean;
	references: Array<Block>;
};

export class File {
	#block: Block;
	#contents: Block;
	#executable: boolean;
	#references: Array<Block>;

	constructor(arg: ConstructorArg) {
		this.#block = arg.block;
		this.#contents = arg.contents;
		this.#executable = arg.executable;
		this.#references = arg.references;
	}

	static async new(...args: Array<Unresolved<File.Arg>>): Promise<File> {
		let {
			contents: contentsArgs,
			executable,
			references,
		} = flatten(
			await Promise.all(
				args.map(async function map(
					unresolvedArg: Unresolved<File.Arg>,
				): Promise<
					MaybeNestedArray<{
						contents: Blob.Arg;
						executable?: boolean;
						references?: Array<Artifact>;
					}>
				> {
					let arg = await resolve(unresolvedArg);
					if (Blob.Arg.is(arg)) {
						return { contents: arg };
					} else if (File.is(arg)) {
						return {
							contents: arg.#contents,
							executable: arg.#executable,
							references: await arg.references(),
						};
					} else if (arg instanceof Array) {
						return await Promise.all(arg.map(map));
					} else if (arg instanceof Object) {
						return {
							contents: arg.contents,
							executable: arg.executable,
							references: arg.references,
						};
					} else {
						return unreachable();
					}
				}),
			),
		).reduce<{
			contents: Array<Blob.Arg>;
			executable: boolean;
			references: Array<Artifact>;
		}>(
			(value, { contents, executable, references }) => {
				value.contents.push(contents);
				value.executable =
					executable !== undefined ? executable : value.executable;
				value.references.push(...(references ?? []));
				return value;
			},
			{ contents: [], executable: false, references: [] },
		);
		let contents = await blob(...contentsArgs);
		return await syscall.file.new({
			contents,
			executable,
			references,
		});
	}

	static is(value: unknown): value is File {
		return value instanceof File;
	}

	static expect(value: unknown): File {
		assert_(File.is(value));
		return value;
	}

	static assert(value: unknown): asserts value is File {
		assert_(File.is(value));
	}

	id(): Id {
		return this.block().id();
	}

	block(): Block {
		return this.#block;
	}

	async contents(): Promise<Blob> {
		return await Blob.withBlock(this.#contents);
	}

	executable(): boolean {
		return this.#executable;
	}

	async references(): Promise<Array<Artifact>> {
		return await Promise.all(this.#references.map(Artifact.withBlock));
	}

	async bytes(): Promise<Uint8Array> {
		return await (await this.contents()).bytes();
	}

	async text(): Promise<string> {
		return await (await this.contents()).text();
	}
}

export namespace File {
	export type Arg = Blob.Arg | File | Array<Arg> | ArgObject;

	export type ArgObject = {
		contents: Blob.Arg;
		executable?: boolean;
		references?: Array<Artifact>;
	};
}
