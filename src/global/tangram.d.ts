/// <reference lib="es2022" />

declare let console: {
	/** Write to the log. */
	log: (...args: Array<unknown>) => void;
};

/**
 * Create a Tangram template with a JavaScript tagged template.
 */
declare var t: (
	strings: TemplateStringsArray,
	...placeholders: Array<tg.Unresolved<tg.TemplateLike>>
) => Promise<tg.Template>;

declare namespace tg {
	// Artifact.

	export type Artifact = Directory | File | Symlink | Reference;

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

	export class Directory {
		/** Get the hash of this `Directory` artifact. This hash covers all child and ancestor artifacts. */
		hash(): Promise<string>;

		/** Try to get the child at the specified path. This method returns `null` if the path does not exist. */
		tryGet(name: PathLike): Promise<Artifact | null>;

		/** Get the child at the specified path. This method throws an error if the path does not exist. */
		get(name: PathLike): Promise<Artifact>;

		/** Get this directory's entries. */
		getEntries(): Promise<Record<string, Artifact>>;

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

	export type FileLike = Uint8Array | string | File;

	type FileOptions = {
		executable?: boolean;
	};

	export let file: (fileLike: FileLike, options?: FileOptions) => Promise<File>;

	export class File {
		/** Get the hash of this `File` artifact. This hash covers the contents of the file. */
		hash(): Promise<string>;

		/** Get this file's contents as a `Uint8Array`. */
		getBytes(): Promise<Uint8Array>;

		/** Get this file's contents as a string. This method throws an error if the contents are not valid UTF-8. */
		getString(): Promise<string>;

		/** Get this file's executable flag. */
		executable(): boolean;
	}

	// Function.

	/** Create a Tangram function. */
	let function_: <A extends Array<Value>, R extends Value>(
		f: (...args: A) => MaybePromise<R>,
	) => (...args: { [K in keyof A]: Unresolved<A[K]> }) => Promise<R>;
	export { function_ as function };

	// Artifact type guards.

	/** Check if a value is an `Artifact`. */
	export let isArtifact: (value: unknown) => value is Artifact;

	/** Check if a value is a `Directory`. */
	export let isDirectory: (value: unknown) => value is Directory;

	/** Check if a value is a `File`. */
	export let isFile: (value: unknown) => value is File;

	/** Check if a value is a `Placeholder`. */
	export let isPlaceholder: (value: unknown) => value is Placeholder;

	/** Check if a value is a `Reference`. */
	export let isReference: (value: unknown) => value is Reference;

	/** Check if a value is a `Template`. */
	export let isTemplate: (value: unknown) => value is Template;

	// Log.

	/** Write to the log. */
	export let log: (...args: Array<unknown>) => void;

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

		/** Normalize this path to produce a path with no redundant `.` and `..` components. Note that normalization does not follow symlinks. */
		normalize(): Path;

		/** Render this path to a string. */
		toString(): string;
	}

	// Placeholder.

	/** Create a placeholder. */
	export let placeholder: (name: string) => Placeholder;

	export class Placeholder {
		/** Get this placeholder's name. */
		name(): string;
	}

	// Process.

	type ProcessArgs = {
		system: System;
		env?: Record<string, TemplateLike> | nullish;
		command: TemplateLike;
		args?: Array<TemplateLike> | nullish;
		unsafe?: boolean | nullish;
	};

	export type System =
		| "amd64_linux"
		| "arm64_linux"
		| "amd64_macos"
		| "arm64_macos";

	export let process: (args: Unresolved<ProcessArgs>) => Promise<Artifact>;

	export let output: tg.Placeholder;

	// Reference.

	type ReferenceArgs = {
		artifact: Unresolved<Artifact>;
		path?: PathLike | nullish;
	};

	/** Create a reference. */
	export let reference: (args: ReferenceArgs) => Promise<Reference>;

	export class Reference {
		/** Get the hash of this `Reference` artifact. This hash covers the referenced artifact plus the reference's path. */
		hash(): Promise<string>;

		/** Get this reference's artifact. */
		getArtifact(): Promise<Artifact>;

		/** Get this reference's path. */
		path(): Path | nullish;
	}

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
		? MaybeThunk<MaybePromise<T>>
		: T extends Array<infer U extends Value>
		? MaybeThunk<MaybePromise<Array<Unresolved<U>>>>
		: T extends { [key: string]: Value }
		? MaybeThunk<MaybePromise<{ [K in keyof T]: Unresolved<T[K]> }>>
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
		: T extends (() => infer U extends Unresolved<Value>)
		? Resolved<U>
		: T extends Promise<infer U extends Unresolved<Value>>
		? Resolved<U>
		: never;

	/** Resolve all deeply nested thunks and promises in an unresolved value. */
	export let resolve: <T extends Unresolved<Value>>(
		value: T,
	) => Promise<Resolved<T>>;

	export type MaybeThunk<T> = T | (() => T);

	export type MaybePromise<T> = T | PromiseLike<T>;

	export type MaybeArray<T> = T | Array<T>;

	// Symlink.

	/** Create a symlink. */
	export let symlink: (target: string) => Symlink;

	export class Symlink {
		/** Get the hash of this `Symlink` artifact. This hash only covers the symlink itself, not the filesystem object it targets. */
		hash(): Promise<string>;

		/** Get this symlink's target. */
		target(): string;
	}

	// Task.

	type TaskArgs = {
		shell?: Function;
		pre?: MaybeArray<Task>;
		post?: MaybeArray<Task>;
		interpreter?: string;
		script: string;
	};

	class Task {}

	export let task: (args: string | TaskArgs) => Task;

	// Template.

	export type TemplateComponent = string | Artifact | Placeholder;

	export type TemplateLike = TemplateComponent | Template | Array<TemplateLike>;

	/** Create a template. */
	export let template: (
		components: tg.Unresolved<tg.TemplateLike>,
	) => Promise<Template>;

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
