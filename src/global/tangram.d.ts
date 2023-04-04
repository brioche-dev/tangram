/// <reference lib="es2022" />

/**
 * Create a Tangram template with a JavaScript tagged template.
 */
declare var t: (
	strings: TemplateStringsArray,
	...placeholders: Array<tg.Unresolved<tg.TemplateLike>>
) => Promise<tg.Template>;

declare namespace tg {
	// Array.

	export type MaybeArray<T> = T | Array<T>;

	export type ArrayLike<T> = Iterable<T>;

	export let array: <T>(value: ArrayLike<T>) => Array<T>;

	// Artifact.

	// eslint-disable-next-line
	const artifactHashSymbol: unique symbol;
	export type ArtifactHash = string & { [artifactHashSymbol]: unknown };

	export type Artifact = Directory | File | Symlink;

	/** Check if a value is an `Artifact`. */
	export let isArtifact: (value: unknown) => value is Artifact;

	export let getArtifact: (hash: ArtifactHash) => Artifact;

	// Blob.

	export type BlobHash = string;

	export type BlobLike = Uint8Array | string | Blob;

	export let isBlobLike: (value: unknown) => value is BlobLike;

	/** Create a blob. */
	export let blob: (blobLike: MaybePromise<BlobLike>) => Promise<Blob>;

	export class Blob {
		/** Get this blob's hash. */
		hash(): BlobHash;

		/** Get this blob's contents as a `Uint8Array`. */
		bytes(): Promise<Uint8Array>;

		/** Get this blob's contents as a string. */
		text(): Promise<string>;
	}

	// Bundle.

	/** Bundle an artifact. */
	export let bundle: (artifact: Unresolved<Directory>) => Promise<Directory>;

	// Checksum.

	export type Checksum = `${ChecksumAlgorithm}${":" | "-"}${string}`;

	export type ChecksumAlgorithm = "blake3" | "sha256";

	export let checksum: (
		algorithm: ChecksumAlgorithm,
		bytes: Uint8Array | string,
	) => Checksum;

	// Context.

	export let context: Map<string, Value>;

	// Directory.

	type DirectoryArg = MaybePromise<nullish | Directory | DirectoryObject>;

	type DirectoryObject = {
		[key: string]: MaybePromise<
			nullish | Uint8Array | string | Artifact | DirectoryObject
		>;
	};

	/** Create a directory. */
	export let directory: (...args: Array<DirectoryArg>) => Promise<Directory>;

	/** Check if a value is a `Directory`. */
	export let isDirectory: (value: unknown) => value is Directory;

	export class Directory {
		/** Get this directory's artifact hash. */
		hash(): Promise<ArtifactHash>;

		/** Try to get the child at the specified path. This method returns `undefined` if the path does not exist. */
		tryGet(pathLike: PathLike): Promise<Artifact | undefined>;

		/** Get the child at the specified path. This method throws an error if the path does not exist. */
		get(pathLike: PathLike): Promise<Artifact>;

		/** Get this directory's entries. */
		entries(): Promise<Map<string, Artifact>>;

		/** Iterate over the names and hashes of this directory's entries. */
		[Symbol.iterator](): Iterator<[string, ArtifactHash]>;

		/** Iterate over this directory's entries. */
		[Symbol.asyncIterator](): AsyncIterator<[string, Artifact]>;
	}

	// Download.

	type DownloadArgs = {
		/** The URL to download from. */
		url: string;

		/** Pass true to choose the format automatically based on the extension in the URL. */
		unpack?: boolean | nullish;

		checksum?: Checksum | nullish;

		unsafe?: boolean | nullish;
	};

	export type UnpackFormat =
		| ".bz2"
		| ".gz"
		| ".lz"
		| ".xz"
		| ".zstd"
		| ".tar"
		| ".tar.bz2"
		| ".tar.gz"
		| ".tar.lz"
		| ".tar.xz"
		| ".tar.zstd"
		| ".zip";

	/** Download an artifact from a URL. */
	export let download: (args: DownloadArgs) => Promise<Artifact>;

	// File.

	export type FileLike = BlobLike | File;

	export let isFileLike: (value: unknown) => value is FileLike;

	export type FileArg = MaybePromise<BlobLike | File | FileObject>;

	export type FileObject = {
		blob: MaybePromise<BlobLike>;
		executable?: boolean;
		references?: Array<MaybePromise<Artifact>>;
	};

	export let file: (arg: FileArg) => Promise<File>;

	/** Check if a value is a `File`. */
	export let isFile: (value: unknown) => value is File;

	export class File {
		/** Get the this file's artifact hash. */
		hash(): Promise<ArtifactHash>;

		/** Get this file's blob. */
		blob(): Blob;

		/** Get this file's contents as a `Uint8Array`. */
		bytes(): Promise<Uint8Array>;

		/** Get this file's contents as a string. This method throws an error if the contents are not valid UTF-8. */
		text(): Promise<string>;

		/** Get this file's executable bit. */
		executable(): boolean;

		/** Get this file's references. */
		references(): Promise<Array<Artifact>>;
	}

	// Function.

	export interface Function<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	> {
		(...args: { [K in keyof A]: Unresolved<A[K]> }): Promise<R>;
	}

	export class Function<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	> extends globalThis.Function {}

	/** Create a Tangram function. */
	let function_: <A extends Array<Value>, R extends Value>(
		f: (...args: A) => MaybePromise<R>,
	) => Function<A, R>;
	export { function_ as function };

	// Include.

	export let include: (path: string) => Promise<Artifact>;

	// Log.

	/** Write to the log. */
	export let log: (...args: Array<unknown>) => void;

	// Map.

	export type MapLike<K extends string, V> = Record<K, V> | Map<K, V>;

	export let map: <K extends string, V>(value: MapLike<K, V>) => Map<K, V>;

	// Path.

	export type PathLike = string | Array<PathComponent> | Path;

	export type PathComponent =
		| { kind: "current_dir" }
		| { kind: "parent_dir" }
		| { kind: "normal"; value: string };

	/** Create a path. */
	export let path: (path: PathLike) => Path;

	export class Path {
		/** Get this paths's components. */
		components(): Array<PathComponent>;

		/** Get this path's parent path. */
		parent(): Path | undefined;

		/** Join this path with another path. */
		join(other: PathLike): Path;

		/** Render this path to a string. */
		toString(): string;
	}

	// Placeholder.

	/** Create a placeholder. */
	export let placeholder: (name: string) => Placeholder;

	/** Check if a value is a `Placeholder`. */
	export let isPlaceholder: (value: unknown) => value is Placeholder;

	export class Placeholder {
		/** Get this placeholder's name. */
		name(): string;
	}

	// Process.

	type ProcessArgs = {
		system: System;
		command: TemplateLike;
		env?: Record<string, TemplateLike> | nullish;
		args?: Array<TemplateLike> | nullish;
		checksum?: Checksum | nullish;
		unsafe?: boolean | nullish;
		network?: boolean | nullish;
		hostPaths?: Array<string> | nullish;
	};

	export type System =
		| "amd64_linux"
		| "arm64_linux"
		| "amd64_macos"
		| "arm64_macos";

	export let process: (args: Unresolved<ProcessArgs>) => Promise<Artifact>;

	export let output: Placeholder;

	// Resolve.

	/**
	 * This computed type takes a type `T` that extends `Value` and returns the union of all possible types that will return `T` by calling `resolve`. Here are some examples:
	 *
	 * ```
	 * Unresolved<string> = MaybePromise<string>
	 * Unresolved<{ key: string }> = MaybePromise<{ key: MaybePromise<string> }>
	 * Unresolved<Array<{ key: string }>> = MaybePromise<Array<MaybePromise<{ key: MaybePromise<string> }>>>
	 * ```
	 */
	export type Unresolved<T extends Value> = T extends
		| nullish
		| boolean
		| number
		| string
		| Artifact
		| Placeholder
		| Template
		? MaybePromise<T>
		: T extends Array<infer U extends Value>
		? MaybePromise<Array<Unresolved<U>>>
		: T extends { [key: string]: Value }
		? MaybePromise<{ [K in keyof T]: Unresolved<T[K]> }>
		: never;

	/**
	 * This computed type performs the inverse operation of `Unresolved`. It takes a type and returns the output of calling `resolve`. Here are some examples:
	 *
	 * ```
	 * Resolved<string> = string
	 * Resolved<() => string> = string
	 * Resolved<Promise<string>> = string
	 * Resolved<Array<Promise<string>>> = Array<string>
	 * Resolved<() => Promise<Array<Promise<string>>>> = Array<string>
	 * Resolved<Promise<Array<Promise<string>>>> = Array<string>
	 * ```
	 */
	export type Resolved<T extends Unresolved<Value>> = T extends
		| nullish
		| boolean
		| number
		| string
		| Artifact
		| Placeholder
		| Template
		? T
		: T extends Array<infer U extends Unresolved<Value>>
		? Array<Resolved<U>>
		: T extends { [key: string]: Unresolved<Value> }
		? { [K in keyof T]: Resolved<T[K]> }
		: T extends Promise<infer U extends Unresolved<Value>>
		? Resolved<U>
		: never;

	/** Resolve all deeply nested thunks and promises in an unresolved value. */
	export let resolve: <T extends Unresolved<Value>>(
		value: T,
	) => Promise<Resolved<T>>;

	export type MaybeThunk<T> = T | (() => T);

	export type MaybePromise<T> = T | PromiseLike<T>;

	// Symlink.

	/** Create a symlink. */
	export let symlink: (target: Unresolved<TemplateLike>) => Promise<Symlink>;

	/** Check if a value is a `Symlink`. */
	export let isSymlink: (value: unknown) => value is Symlink;

	export class Symlink {
		/** Get this symlink's artifact hash. */
		hash(): Promise<ArtifactHash>;

		/** Get this symlink's target. */
		target(): Template;
	}

	// Template.

	export type TemplateComponent = string | Artifact | Placeholder;

	export type TemplateLike = TemplateComponent | Template | Array<TemplateLike>;

	/** Create a template. */
	export let template: (
		components: Unresolved<TemplateLike>,
	) => Promise<Template>;

	/** Check if a value is a `Template`. */
	export let isTemplate: (value: unknown) => value is Template;

	export class Template {
		/** Get this template's components. */
		components(): Array<TemplateComponent>;

		/** Render this template using the provided function that renders each component to a string. */
		render(f: (component: TemplateComponent) => string): string;
	}

	// Value.

	/** A `Value` is the union of all types that can serve as arguments or return values of Tangram functions. */
	export type Value =
		| nullish
		| boolean
		| number
		| string
		| Artifact
		| Placeholder
		| Template
		| Array<Value>
		| { [key: string]: Value };

	export type nullish = undefined | null;
}

declare let console: {
	/** Write to the log. */
	log: (...args: Array<unknown>) => void;
};

interface JSON {
	/**
	 * Converts a JavaScript Object Notation (JSON) string into an object.
	 * @param text A valid JSON string.
	 * @param reviver A function that transforms the results. This function is called for each member of the object.
	 * If a member contains nested objects, the nested objects are transformed before the parent object is.
	 */
	parse(
		text: string,
		reviver?: (this: any, key: string, value: any) => any,
	): unknown;
}
