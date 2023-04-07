/// <reference lib="es2023" />

declare namespace tg {
	// Artifact.

	export type Artifact = Directory | File | Symlink;

	export namespace Artifact {
		export type Hash = string;

		/** Check if a value is an `Artifact`. */
		export let isArtifact: (value: unknown) => value is Artifact;

		/* Get an artifact by its hash. */
		export let get: (hash: Hash) => Promise<Artifact>;
	}

	// Blob.

	export namespace Blob {
		export type Arg = Uint8Array | string | Blob;

		export type Hash = string;
	}

	/** Create a blob. */
	export let blob: (arg: Unresolved<Blob.Arg>) => Promise<Blob>;

	export class Blob {
		/** Check if a value is a `Blob`. */
		static isBlob: (value: unknown) => value is Blob;

		/* Get this blob's hash. */
		hash(): Blob.Hash;

		/** Get this blob's contents as a `Uint8Array`. */
		bytes(): Promise<Uint8Array>;

		/** Get this blob's contents as a string. */
		text(): Promise<string>;
	}

	// Call.

	export namespace Call {
		type Arg<A extends Array<Value>, R extends Value> = {
			function: Function<A, R>;
			env?: Record<string, Value> | nullish;
			args: A;
		};
	}

	/** Call a Tangram function. */
	export let call: <A extends Array<Value>, R extends Value>(
		arg: Call.Arg<A, R>,
	) => Promise<R>;

	// Checksum.

	export type Checksum = string;

	export namespace Checksum {
		export type Algorithm = "blake3" | "sha256";
	}

	// Directory.

	export namespace Directory {
		type Arg = nullish | Directory | ArgObject;

		type ArgObject = { [key: string]: ArgObjectValue };

		type ArgObjectValue = nullish | Blob.Arg | Artifact | ArgObject;
	}

	/** Create a directory. */
	export let directory: (
		...args: Array<Unresolved<Directory.Arg>>
	) => Promise<Directory>;

	/** A directory. */
	export class Directory {
		/** Check if a value is a `Directory`. */
		static isDirectory: (value: unknown) => value is Directory;

		/* Get this directory's hash. */
		hash(): Artifact.Hash;

		/** Get the child at the specified path. This method throws an error if the path does not exist. */
		get(arg: Path.Arg): Promise<Artifact>;

		/** Try to get the child at the specified path. This method returns `undefined` if the path does not exist. */
		tryGet(arg: Path.Arg): Promise<Artifact | undefined>;

		/** Get this directory's entries. */
		entries(): Promise<Map<string, Artifact>>;

		/** Bundle this directory. */
		bundle: () => Promise<Directory>;

		/** Walk this directory's recursive entries. */
		walk(): AsyncIterableIterator<[Path, Artifact]>;

		/** An async iterator of this directory's entries. */
		[Symbol.asyncIterator](): AsyncIterator<[string, Artifact]>;
	}

	// Download.

	export namespace Download {
		export type Arg = {
			/** The URL to download from. */
			url: string;

			/** Pass true to choose the format automatically based on the extension in the URL. */
			unpack?: boolean | nullish;

			checksum?: Checksum | nullish;

			unsafe?: boolean | nullish;
		};
	}

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

	/** Download an artifact. */
	export let download: (arg: Download.Arg) => Promise<Artifact>;

	// Encoding.

	export namespace base64 {
		export let encode: (value: Uint8Array) => string;
		export let decode: (value: string) => Uint8Array;
	}

	export namespace hex {
		export let encode: (value: Uint8Array) => string;
		export let decode: (value: string) => Uint8Array;
	}

	export namespace json {
		export let encode: (value: any) => string;
		export let decode: (value: string) => unknown;
	}

	export namespace toml {
		export let encode: (value: any) => string;
		export let decode: (value: string) => unknown;
	}

	export namespace utf8 {
		export let encode: (value: string) => Uint8Array;
		export let decode: (value: Uint8Array) => string;
	}

	export namespace yaml {
		export let encode: (value: any) => string;
		export let decode: (value: string) => unknown;
	}

	// Env.

	export let env: {
		/** Get the env value with the provided key. This function returns `undefined` if the key is not set in the env. */
		get: (key: string) => Value | undefined;
	};

	// File.

	export namespace File {
		export type Arg = Blob.Arg | File | ArgObject;

		export type ArgObject = {
			blob: Blob.Arg;
			executable?: boolean;
			references?: Array<Artifact>;
		};
	}

	export let file: (arg: Unresolved<File.Arg>) => Promise<File>;

	export class File {
		/** Check if a value is a `File`. */
		static isFile: (value: unknown) => value is File;

		/* Get this file's hash. */
		hash(): Artifact.Hash;

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

	/** Create a Tangram function. */
	let function_: <A extends Array<Value>, R extends Value>(
		f: (...args: A) => MaybePromise<R>,
	) => Function<A, R>;
	export { function_ as function };

	export type Function<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	> = {
		(...args: { [K in keyof A]: Unresolved<A[K]> }): Promise<R>;
	};

	export let Function: {
		isFunction: (value: unknown) => value is Function<any, any>;
	};

	// Include.

	/** Include an artifact from a package, at a path relative to the module this function is called from. The path must be a string literal so that it can be statically analyzed. */
	export let include: (path: string) => Promise<Artifact>;

	// Log.

	/** Write to the log. */
	export let log: (...args: Array<unknown>) => void;

	// Path.

	export namespace Path {
		export type Arg = nullish | string | Path.Component | Path | Array<Arg>;
	}

	/** Create a path. */
	export let path: (...args: Array<Path.Arg>) => Path;

	export class Path {
		/** Get this path's components. */
		components(): Array<Path.Component>;

		/** Join this path with another path. */
		join(other: Path.Arg): Path;

		/** Create a path to this path from `src`. */
		diff(src: Path.Arg): Path;

		/** Render this path to a string. */
		toString(): string;
	}

	export namespace Path {
		export type Component =
			| { kind: "parent" }
			| { kind: "normal"; value: string };

		export namespace Component {
			/** Check if a value is a `Path.Component`. */
			export let isPathComponent: (value: unknown) => value is Path.Component;

			/** Check if two path components are equal. */
			export let equal: (a: Path.Component, b: Path.Component) => boolean;
		}
	}

	// Placeholder.

	/** Create a placeholder. */
	export let placeholder: (name: string) => Placeholder;

	/** A placeholder. */
	export class Placeholder {
		/** Check if a value is a `Placeholder`. */
		static isPlaceholder: (value: unknown) => value is Placeholder;

		/** Get this placeholder's name. */
		name(): string;
	}

	// Process.

	export namespace Process {
		export type Arg = {
			/** The system to run the process on. */
			system: System;

			/** The command to run. */
			executable: Template.Arg;

			/** The environment variables to set for the process. */
			env?: Record<string, Template.Arg> | nullish;

			/** The command line arguments to pass to the process. */
			args?: Array<Template.Arg> | nullish;

			/** A checksum for the process's output. If set, then unsafe options can be used. */
			checksum?: Checksum | nullish;

			/** Use this flag to enable unsafe options without providing a checksum. */
			unsafe?: boolean | nullish;

			/** Whether to enable network access. Because this is an unsafe option, you must either provide a checksum for the process's output or set `unsafe` to `true`. */
			network?: boolean | nullish;

			/** Paths on the host to mount in the sandbox the process runs in. Because this is an unsafe option, you must either provide a checksum for the process's output or set `unsafe` to `true`. */
			hostPaths?: Array<string> | nullish;
		};
	}

	export let process: (arg: Unresolved<Process.Arg>) => Promise<Artifact>;

	export let output: Placeholder;

	// Resolve.

	/**
	 * This computed type takes a type `T` and returns the union of all possible types that will return `T` by calling `resolve`. Here are some examples:
	 *
	 * ```
	 * Unresolved<string> = MaybePromise<string>
	 * Unresolved<{ key: string }> = MaybePromise<{ key: MaybePromise<string> }>
	 * Unresolved<Array<{ key: string }>> = MaybePromise<Array<MaybePromise<{ key: MaybePromise<string> }>>>
	 * ```
	 */
	export type Unresolved<T extends Value> = MaybePromise<
		T extends
			| nullish
			| boolean
			| number
			| string
			| Uint8Array
			| Path
			| Blob
			| Artifact
			| Placeholder
			| Template
			? T
			: T extends Array<infer U extends Value>
			? Array<Unresolved<U>>
			: T extends { [key: string]: Value }
			? { [K in keyof T]: Unresolved<T[K]> }
			: never
	>;

	/**
	 * This computed type performs the inverse operation of `Unresolved`. It takes a type and returns the output of calling `resolve` on a value of that type. Here are some examples:
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
		| Uint8Array
		| Path
		| Blob
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

	/** Resolve all deeply nested promises in an unresolved value. */
	export let resolve: <T extends Unresolved<Value>>(
		value: T,
	) => Promise<Resolved<T>>;

	export type MaybePromise<T> = T | Promise<T>;

	// Symlink.

	export namespace Symlink {
		type Arg = Path.Arg | Artifact | Template | ArgObject;

		type ArgObject = {
			artifact?: Artifact | nullish;
			path?: Path.Arg | nullish;
		};
	}

	/** Create a symlink. */
	export let symlink: (target: Unresolved<Symlink.Arg>) => Promise<Symlink>;

	export class Symlink {
		/** Check if a value is a `Symlink`. */
		static isSymlink: (value: unknown) => value is Symlink;

		/* Get this symlink's hash. */
		hash(): Artifact.Hash;

		/** Get this symlink's target. */
		target(): Template;

		/** Get this symlink's artifact. */
		artifact(): Artifact | undefined;

		/** Get this symlink's path. */
		path(): Path;

		/** Resolve this symlink to the directory or file it refers to, or return undefined if none is found. */
		resolve(): Promise<Directory | File | undefined>;
	}

	// System.

	export namespace System {
		export type Arg = System | ArgObject;

		export type ArgObject = {
			arch: System.Arch;
			os: System.Os;
		};
	}

	/** Create a system. */
	export let system: (arg: System.Arg) => System;

	export type System =
		| "amd64_linux"
		| "arm64_linux"
		| "amd64_macos"
		| "arm64_macos";

	export namespace System {
		export type Arch = "amd64" | "arm64";

		export type Os = "linux" | "macos";

		/** Check if a value is a `System`. */
		export let isSystem: (value: unknown) => value is System;

		/** Get a system's arch. */
		export let arch: (value: System) => Arch;

		/** Get a system's OS. */
		export let os: (value: System) => Os;
	}

	// Template.

	export namespace Template {
		export type Arg = Template.Component | Path | Template | Array<Arg>;
	}

	/** Create a template. */
	export let template: (
		...args: Array<Unresolved<Template.Arg | nullish>>
	) => Promise<Template>;

	export class Template {
		/** Check if a value is a `Template`. */
		static isTemplate: (value: unknown) => value is Template;

		/** Join an array of templates with a separator. */
		static join(
			separator: Template.Arg,
			...args: Array<Template.Arg | nullish>
		): Promise<Template>;

		/** Get this template's components. */
		components(): Array<Template.Component>;
	}

	export namespace Template {
		export type Component = string | Artifact | Placeholder;

		export namespace Component {
			/** Check if a value is a `Template.Component`. */
			export let isTemplateComponent: (value: unknown) => value is Component;
		}
	}

	// Value.

	/** A `Value` is the union of all types that can be used as arguments or return values of Tangram functions. */
	export type Value =
		| nullish
		| boolean
		| number
		| string
		| Uint8Array
		| Path
		| Blob
		| Artifact
		| Placeholder
		| Template
		| Array<Value>
		| { [key: string]: Value };

	export type nullish = undefined | null;

	export namespace nullish {
		export let isNullish: (value: unknown) => value is nullish;
	}
}

/**
 * Create a Tangram template with a JavaScript tagged template.
 */
declare var t: (
	strings: TemplateStringsArray,
	...placeholders: Array<tg.Unresolved<tg.Template.Arg | tg.nullish>>
) => Promise<tg.Template>;

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
