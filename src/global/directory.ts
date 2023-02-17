import {
	Artifact,
	ArtifactHash,
	addArtifact,
	getArtifact,
	isArtifact,
} from "./artifact";
import { file, isFileLike } from "./file";
import { PathLike, path } from "./path";
import { MaybePromise } from "./resolve";
import { assert } from "./util";
import { isNullish, nullish } from "./value";

type DirectoryArg = MaybePromise<nullish | Directory | DirectoryObject>;

type DirectoryObject = {
	[name: string]: MaybePromise<
		nullish | Uint8Array | string | Artifact | DirectoryObject
	>;
};

export let directory = async (
	...args: Array<DirectoryArg>
): Promise<Directory> => {
	let entries: Map<string, ArtifactHash> = new Map();

	// Apply each arg.
	for (let arg of args) {
		arg = await arg;
		if (isNullish(arg)) {
			// If the arg is null, then continue.
		} else if (arg instanceof Directory) {
			// If the arg is a directory, then apply each entry.
			for (let [name, hash] of arg) {
				entries.set(name, hash);
			}
		} else {
			// If the arg is an object, then apply each entry.
			for (let [key, value] of Object.entries(arg)) {
				// Normalize the path and separate the first path component from the trailing path components.
				let [firstComponent, ...trailingComponents] = path(key)
					.normalize()
					.components();

				// All path components must be normal.
				if (firstComponent.kind !== "normal") {
					throw new Error(`Invalid path component.`);
				}
				let name = firstComponent.value;

				if (trailingComponents.length > 0) {
					// If there are trailing path components, then recurse.
					let trailingPath = path(trailingComponents).toString();

					// Get an existing directory.
					let entryHash = entries.get(name);
					let entry;
					if (entryHash !== undefined) {
						// Get the entry artifact.
						entry = await getArtifact(entryHash);

						// Ensure the entry is a directory.
						if (!(entry instanceof Directory)) {
							entry = undefined;
						}
					}

					// Merge the entry with the trailing path.
					let child = await directory(entry, {
						[trailingPath]: value,
					});

					entries.set(name, await addArtifact(child));
				} else {
					// If there are no trailing path components, then create the artifact specified by the value.
					value = await value;
					if (isNullish(value)) {
						entries.delete(name);
					} else if (isFileLike(value)) {
						entries.set(name, await addArtifact(await file(value)));
					} else if (isArtifact(value)) {
						entries.set(name, await addArtifact(value));
					} else {
						entries.set(name, await addArtifact(await directory(value)));
					}
				}
			}
		}
	}

	return new Directory(entries);
};

export let isDirectory = (value: unknown): value is Directory => {
	return value instanceof Directory;
};

export class Directory {
	#entries: Map<string, ArtifactHash>;

	constructor(entries: Map<string, ArtifactHash>) {
		this.#entries = entries;
	}

	static async fromHash(hash: ArtifactHash): Promise<Directory> {
		let artifact = await getArtifact(hash);
		assert(isDirectory(artifact));
		return artifact;
	}

	async serialize(): Promise<syscall.Directory> {
		let entries = Object.fromEntries(Array.from(this.#entries.entries()));
		return {
			entries,
		};
	}

	static async deserialize(directory: syscall.Directory): Promise<Directory> {
		let entries = new Map(Object.entries(directory.entries));
		return new Directory(entries);
	}

	async tryGet(name: PathLike): Promise<Artifact | null> {
		// Normalize the path and separate the first path component from the trailing path components.
		let [firstComponent, ...trailingComponents] = path(name)
			.normalize()
			.components();

		// All path components must be normal.
		if (firstComponent.kind !== "normal") {
			throw new Error(`Invalid path "${name}".`);
		}

		// Get the next entry.
		let hash = this.#entries.get(firstComponent.value);
		if (!hash) {
			return null;
		}
		let artifact = await getArtifact(hash);

		if (trailingComponents.length > 0) {
			// If there are trailing path components, then recurse.
			assert(isDirectory(artifact));
			return await artifact.tryGet(trailingComponents);
		} else {
			return artifact;
		}
	}

	async get(name: PathLike): Promise<Artifact> {
		let artifact = await this.tryGet(name);
		if (artifact === null) {
			throw new Error(`Failed to get directory entry "${name}".`);
		}
		return artifact;
	}

	async getEntries(): Promise<Record<string, Artifact>> {
		let entries: Record<string, Artifact> = {};
		for await (let [name, artifact] of this) {
			entries[name] = await artifact;
		}
		return entries;
	}

	*[Symbol.iterator](): Iterator<[string, ArtifactHash]> {
		for (let [name, entry] of this.#entries) {
			yield [name, entry];
		}
	}

	async *[Symbol.asyncIterator](): AsyncIterator<[string, Artifact]> {
		for (let name of this.#entries.keys()) {
			yield [name, await this.get(name)];
		}
	}
}
