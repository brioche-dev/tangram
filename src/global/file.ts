import { Artifact } from "./artifact.ts";
import { assert as assert_ } from "./assert.ts";
import { Blob, blob } from "./blob.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";

export let file = async (arg: Unresolved<File.Arg>) => {
	return await File.new(arg);
};

type ConstructorArg = {
	hash: Artifact.Hash;
	blob: Blob;
	executable: boolean;
	references: Array<Artifact.Hash>;
};

export class File {
	#hash: Artifact.Hash;
	#blob: Blob;
	#executable: boolean;
	#references: Array<Artifact.Hash>;

	static async new(arg: Unresolved<File.Arg>): Promise<File> {
		// Resolve the arg.
		let resolvedArg = await resolve(arg);

		// Get the blob, executable, and references.
		let blob_: Blob;
		let executable: boolean;
		let references: Array<Artifact>;
		if (Blob.Arg.is(resolvedArg)) {
			// If the arg is a blob arg, then create a file which is not executable and has no references.
			blob_ = await blob(resolvedArg);
			executable = false;
			references = [];
		} else if (File.is(resolvedArg)) {
			// If the arg is a file, then return it.
			return resolvedArg;
		} else {
			// Otherwise, the arg is a file object.
			blob_ = await blob(resolvedArg.blob);
			executable = resolvedArg.executable ?? false;
			references = resolvedArg.references ?? [];
		}

		// Create the file.
		return File.fromSyscall(
			await syscall.file.new({
				blob: blob_.toSyscall(),
				executable,
				references: references.map((reference) =>
					Artifact.toSyscall(reference),
				),
			}),
		);
	}

	constructor(arg: ConstructorArg) {
		this.#hash = arg.hash;
		this.#blob = arg.blob;
		this.#executable = arg.executable;
		this.#references = arg.references;
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

	toSyscall(): syscall.File {
		return {
			hash: this.#hash,
			blob: this.#blob.toSyscall(),
			executable: this.#executable,
			references: this.#references,
		};
	}

	static fromSyscall(value: syscall.File): File {
		return new File({
			hash: value.hash,
			blob: Blob.fromSyscall(value.blob),
			executable: value.executable,
			references: value.references,
		});
	}

	hash(): Artifact.Hash {
		return this.#hash;
	}

	blob(): Blob {
		return this.#blob;
	}

	executable(): boolean {
		return this.#executable;
	}

	async references(): Promise<Array<Artifact>> {
		return await Promise.all(this.#references.map(Artifact.get));
	}

	async bytes(): Promise<Uint8Array> {
		return await this.blob().bytes();
	}

	async text(): Promise<string> {
		return await this.blob().text();
	}
}

export namespace File {
	export type Arg = Blob.Arg | File | ArgObject;

	export type ArgObject = {
		blob: Blob.Arg;
		executable?: boolean;
		references?: Array<Artifact>;
	};
}
