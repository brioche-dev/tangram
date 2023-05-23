import { Artifact } from "./artifact.ts";
import { assert as assert_ } from "./assert.ts";
import { Blob } from "./blob.ts";
import { File, file } from "./file.ts";
import { Subpath, subpath } from "./path.ts";
import { Unresolved, resolve } from "./resolve.ts";
import { Symlink } from "./symlink.ts";
import * as syscall from "./syscall.ts";

export let directory = async (...args: Array<Unresolved<Directory.Arg>>) => {
	return await Directory.new(...args);
};

type ConstructorArg = {
	hash: Artifact.Hash;
	entries: Record<string, Artifact.Hash>;
};

export class Directory {
	#hash: Artifact.Hash;
	#entries: Record<string, Artifact.Hash>;

	static async new(
		...args: Array<Unresolved<Directory.Arg>>
	): Promise<Directory> {
		// Create the entries.
		let entries: Record<string, Artifact> = {};

		// Apply each arg.
		for (let arg of await Promise.all(args.map(resolve))) {
			if (arg === undefined) {
				// If the arg is undefined, then continue.
			} else if (arg instanceof Directory) {
				// If the arg is a directory, then apply each entry.
				for (let [name, entry] of Object.entries(await arg.entries())) {
					// Get an existing entry.
					let existingEntry = entries[name];

					// Merge the existing entry with the entry if they are both directories.
					if (
						existingEntry instanceof Directory &&
						entry instanceof Directory
					) {
						entry = await Directory.new(existingEntry, entry);
					}

					// Set the entry.
					entries[name] = entry;
				}
			} else if (typeof arg === "object") {
				// If the arg is an object, then apply each entry.
				for (let [key, value] of Object.entries(arg)) {
					// Separate the first path component from the trailing path components.
					let [firstComponent, ...trailingComponents] =
						subpath(key).components();
					if (firstComponent === undefined) {
						throw new Error("The path must have at least one component.");
					}
					let name = firstComponent;

					// Get an existing entry.
					let existingEntry = entries[name];

					// Remove the entry if it is not a directory.
					if (!(existingEntry instanceof Directory)) {
						existingEntry = undefined;
					}

					if (trailingComponents.length > 0) {
						// If there are trailing path components, then recurse.
						let trailingPath = subpath(trailingComponents).toString();

						// Merge the entry with the trailing path.
						let newEntry = await Directory.new(existingEntry, {
							[trailingPath]: value,
						});

						// Add the entry.
						entries[name] = newEntry;
					} else {
						// If there are no trailing path components, then create the artifact specified by the value.
						if (value === undefined) {
							delete entries[name];
						} else if (Blob.Arg.is(value)) {
							let newEntry = await file(value);
							entries[name] = newEntry;
						} else if (File.is(value) || Symlink.is(value)) {
							entries[name] = value;
						} else {
							let newEntry = await Directory.new(existingEntry, value);
							entries[name] = newEntry;
						}
					}
				}
			}
		}

		// Create the directory.
		return Directory.fromSyscall(
			await syscall.directory.new({
				entries: Object.fromEntries(
					Object.entries(entries).map(([name, entry]) => [
						name,
						Artifact.toSyscall(entry),
					]),
				),
			}),
		);
	}

	constructor(arg: ConstructorArg) {
		this.#hash = arg.hash;
		this.#entries = arg.entries;
	}

	static is(value: unknown): value is Directory {
		return value instanceof Directory;
	}

	static expect(value: unknown): Directory {
		assert_(Directory.is(value));
		return value;
	}

	static assert(value: unknown): asserts value is Directory {
		assert_(Directory.is(value));
	}

	toSyscall(): syscall.Directory {
		return {
			hash: this.#hash,
			entries: this.#entries,
		};
	}

	static fromSyscall(directory: syscall.Directory): Directory {
		let hash = directory.hash;
		let entries = directory.entries;
		return new Directory({ hash, entries });
	}

	hash(): Artifact.Hash {
		return this.#hash;
	}

	async get(arg: Subpath.Arg): Promise<Artifact> {
		let artifact = await this.tryGet(arg);
		assert_(artifact, `Failed to get the directory entry "${arg}".`);
		return artifact;
	}

	async tryGet(arg: Subpath.Arg): Promise<Artifact | undefined> {
		let artifact: Artifact = this;
		for (let component of subpath(arg).components()) {
			if (!(artifact instanceof Directory)) {
				return undefined;
			}
			let hash = artifact.#entries[component];
			if (!hash) {
				return undefined;
			}
			artifact = await Artifact.get(hash);
		}
		return artifact;
	}

	async entries(): Promise<Record<string, Artifact>> {
		let entries: Record<string, Artifact> = {};
		for await (let [name, artifact] of this) {
			entries[name] = artifact;
		}
		return entries;
	}

	async bundle(): Promise<Directory> {
		let bundledArtifact = Artifact.fromSyscall(
			await syscall.artifact.bundle(Artifact.toSyscall(this)),
		);
		assert_(Directory.is(bundledArtifact));
		return bundledArtifact;
	}

	async *walk(): AsyncIterableIterator<[Subpath, Artifact]> {
		for await (let [name, artifact] of this) {
			yield [subpath(name), artifact];
			if (Directory.is(artifact)) {
				for await (let [entryName, entryArtifact] of artifact.walk()) {
					yield [subpath(name).join(entryName), entryArtifact];
				}
			}
		}
	}

	*[Symbol.iterator](): Iterator<[string, Artifact.Hash]> {
		for (let [name, entry] of Object.entries(this.#entries)) {
			yield [name, entry];
		}
	}

	async *[Symbol.asyncIterator](): AsyncIterator<[string, Artifact]> {
		for (let name of Object.keys(this.#entries)) {
			yield [name, await this.get(name)];
		}
	}
}

export namespace Directory {
	export type Arg = undefined | Directory | ArgObject;

	export type ArgObject = { [name: string]: ArgObjectValue };

	export type ArgObjectValue = undefined | Blob.Arg | Artifact | ArgObject;
}
