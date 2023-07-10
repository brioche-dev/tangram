/// <reference lib="es2023" />

declare namespace tg {
	// Assertions.
	export let assert: (
		condition: unknown,
		message?: string,
	) => asserts condition;

	export let unimplemented: (message?: string) => never;

	export let unreachable: (message?: string) => never;

	// Artifact.

	/** An artifact. */
	export type Artifact = Directory | File | Symlink;

	export namespace Artifact {
		/** An artifact hash. */
		export type Hash = string;

		/** Check if a value is an `Artifact`. */
		export let is: (value: unknown) => value is Artifact;

		/** Expect that a value is an `Artifact`. */
		export let expect: (value: unknown) => Artifact;

		/** Assert that a value is an `Artifact`. */
		export let assert: (value: unknown) => asserts value is Artifact;

		/* Get an artifact by its hash. */
		export let get: (hash: Hash) => Promise<Artifact>;
	}

	// Blob.

	/** Create a blob. */
	export let blob: (arg: Unresolved<Blob.Arg>) => Promise<Blob>;

	export class Blob {
		/** Create a blob. */
		static new(arg: Unresolved<Blob.Arg>): Promise<Blob>;

		/** Check if a value is a `Blob`. */
		static is(value: unknown): value is Blob;

		/** Expect that a value is a `Blob`. */
		static expect: (value: unknown) => Blob;

		/** Assert that a value is a `Blob`. */
		static assert: (value: unknown) => asserts value is Blob;

		/* Get this blob's hash. */
		hash(): Blob.Hash;

		/** Get this blob's contents as a `Uint8Array`. */
		bytes(): Promise<Uint8Array>;

		/** Get this blob's contents as a string. */
		text(): Promise<string>;
	}

	export namespace Blob {
		export type Hash = string;

		export type Arg = Uint8Array | string | Blob;
	}

	// Checksum.

	/** Compute the checksum of the provided bytes. */
	export let checksum: (
		algorithm: Checksum.Algorithm,
		bytes: Uint8Array,
	) => Checksum;

	export type Checksum = string;

	export namespace Checksum {
		export type Algorithm = "blake3" | "sha256" | "sha512";

		export let new_: (algorithm: Algorithm, bytes: Uint8Array) => Checksum;
		export { new_ as new };
	}

	// Directory.

	/** Create a directory. */
	export let directory: (
		...args: Array<Unresolved<Directory.Arg>>
	) => Promise<Directory>;

	/** A directory. */
	export class Directory {
		/** Create a directory. */
		static new: (
			...args: Array<Unresolved<Directory.Arg>>
		) => Promise<Directory>;

		/** Check if a value is a `Directory`. */
		static is: (value: unknown) => value is Directory;

		/** Expect that a value is a `Directory`. */
		static expect: (value: unknown) => Directory;

		/** Assert that a value is a `Directory`. */
		static assert: (value: unknown) => asserts value is Directory;

		/* Get this directory's hash. */
		hash(): Artifact.Hash;

		/** Get the child at the specified path. This method throws an error if the path does not exist. */
		get(arg: Subpath.Arg): Promise<Artifact>;

		/** Try to get the child at the specified path. This method returns `undefined` if the path does not exist. */
		tryGet(arg: Subpath.Arg): Promise<Artifact | undefined>;

		/** Get this directory's entries. */
		entries(): Promise<Record<string, Artifact>>;

		/** Bundle this directory with all its recursive references. */
		bundle: () => Promise<Directory>;

		/** Get an iterator of this directory's recursive entries. */
		walk(): AsyncIterableIterator<[Subpath, Artifact]>;

		/** Get an async iterator of this directory's entries. */
		[Symbol.asyncIterator](): AsyncIterator<[string, Artifact]>;
	}

	export namespace Directory {
		type Arg = undefined | Directory | ArgObject;

		type ArgObject = { [key: string]: ArgObjectValue };

		type ArgObjectValue = undefined | Blob.Arg | Artifact | ArgObject;
	}

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
		export let encode: (value: unknown) => string;
		export let decode: (value: string) => unknown;
	}

	export namespace toml {
		export let encode: (value: unknown) => string;
		export let decode: (value: string) => unknown;
	}

	export namespace utf8 {
		export let encode: (value: string) => Uint8Array;
		export let decode: (value: Uint8Array) => string;
	}

	export namespace yaml {
		export let encode: (value: unknown) => string;
		export let decode: (value: string) => unknown;
	}

	// Env.

	export let env: {
		get: () => Record<string, Value>;
	};

	// File.

	/** Create a file. */
	export let file: (arg: Unresolved<File.Arg>) => Promise<File>;

	/** A file. */
	export class File {
		/** Create a file. */
		static new: (arg: Unresolved<File.Arg>) => Promise<File>;

		/** Check if a value is a `File`. */
		static is: (value: unknown) => value is File;

		/** Expect that a value is a `File`. */
		static expect: (value: unknown) => File;

		/** Assert that a value is a `File`. */
		static assert: (value: unknown) => asserts value is File;

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

	export namespace File {
		export type Arg = Blob.Arg | File | ArgObject;

		export type ArgObject = {
			blob: Blob.Arg;
			executable?: boolean;
			references?: Array<Artifact>;
		};
	}

	// Function.

	/** Create a Tangram function. */
	function function_<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	>(f: (...args: A) => MaybePromise<R | void>): Function<A, R>;
	function function_<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	>(name: string, f: (...args: A) => MaybePromise<R | void>): Function<A, R>;
	export { function_ as function };

	/** A Tangram function. */
	export interface Function<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	> {
		/** Call this function. */
		(...args: { [K in keyof A]: Unresolved<A[K]> }): Promise<R>;
	}

	export class Function<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	> extends globalThis.Function {
		/** Check if a value is a `Function`. */
		static is(value: unknown): value is Function;

		/** Expect that a value is a `Function`. */
		static expect(value: unknown): Function;

		/** Assert that a value is a `Function`. */
		static assert(value: unknown): asserts value is Function;

		/** Get this function's hash. */
		hash(): Operation.Hash;

		/** Get this function's env. */
		env(): Record<string, Value>;

		/** Get this function's args. */
		args(): Array<Value>;
	}

	// Include.

	/** Include an artifact at a path relative to the module this function is called from. The path must be a string literal so that it can be statically analyzed. */
	export let include: (path: string) => Promise<Artifact>;

	// Log.

	/** Write to the log. */
	export let log: (...args: Array<unknown>) => void;

	// Operation.

	export type Operation = Command | Function | Resource;

	export namespace Operation {
		export type Hash = string;
	}

	// Path.

	/** Create a relative path. */
	export let relpath: (...args: Array<Relpath.Arg>) => Relpath;

	/** A relative path. */
	export class Relpath {
		/** Create a new relpath. */
		static new: (...args: Array<Relpath.Arg>) => Relpath;

		/** Check if a value is a `Relpath`. */
		static is: (value: unknown) => value is Relpath;

		/** Expect that a value is a `Relpath`. */
		static expect: (value: unknown) => Relpath;

		/** Assert that a value is a `Relpath`. */
		static assert: (value: unknown) => asserts value is Relpath;

		/** Get this relpath's parents. */
		parents(): number;

		/** Get this relpath's subpath. */
		subpath(): Subpath;

		/** Join this relpath with another relpath. */
		join(other: Relpath.Arg): Relpath;

		/** Render this relpath to a string. */
		toString(): string;
	}

	export namespace Relpath {
		export type Arg = undefined | string | Subpath | Relpath | Array<Arg>;
	}

	/** Create a subpath. */
	export let subpath: (...args: Array<Subpath.Arg>) => Subpath;

	/** A subpath. */
	export class Subpath {
		/** Create a new subpath. */
		static new: (...args: Array<Subpath.Arg>) => Subpath;

		/** Check if a value is a `Subpath`. */
		static is: (value: unknown) => value is Subpath;

		/** Expect that a value is a `Subpath`. */
		static expect: (value: unknown) => Subpath;

		/** Assert that a value is a `Subpath`. */
		static assert: (value: unknown) => asserts value is Subpath;

		/** Get this subpath's components. */
		components(): Array<string>;

		/** Join this subpath with another subpath. */
		join(other: Subpath.Arg): Subpath;

		/** Render this subpath to a string. */
		toString(): string;
	}

	export namespace Subpath {
		export type Arg = undefined | string | Subpath | Array<Arg>;
	}

	// Placeholder.

	/** Create a placeholder. */
	export let placeholder: (name: string) => Placeholder;

	/** A placeholder. */
	export class Placeholder {
		/** Create a new placeholder. */
		static new: (name: string) => Placeholder;

		/** Check if a value is a `Placeholder`. */
		static is: (value: unknown) => value is Placeholder;

		/** Expect that a value is a `Placeholder`. */
		static expect: (value: unknown) => Placeholder;

		/** Assert that a value is a `Placeholder`. */
		static assert: (value: unknown) => asserts value is Placeholder;

		/** Get this placeholder's name. */
		name(): string;
	}

	// Command.

	/** Create a command. */
	export let command: (arg: Unresolved<Command.Arg>) => Promise<Command>;

	/** Run a command. */
	export let run: (
		arg: Unresolved<Command.Arg>,
	) => Promise<Artifact | undefined>;

	export let output: Placeholder;

	/** A command. */
	export class Command {
		/** Create a command. */
		static new: (target: Unresolved<Command.Arg>) => Promise<Command>;

		/** Check if a value is a `Command`. */
		static is: (value: unknown) => value is Command;

		/** Expect that a value is a `Command`. */
		static expect: (value: unknown) => Command;

		/** Assert that a value is a `Command`. */
		static assert: (value: unknown) => asserts value is Command;

		/** Get this command's hash. */
		hash(): Operation.Hash;

		/** Run this command. */
		run(): Promise<Artifact | undefined>;
	}

	export namespace Command {
		export type Arg = {
			/** The system to run the command on. */
			system: System;

			/** The executable to run. */
			executable: Template.Arg;

			/** The environment variables to set for the command. */
			env?: Record<string, Template.Arg>;

			/** The command line arguments to pass to the command. */
			args?: Array<Template.Arg>;

			/** A checksum for the command's output. If a checksum is provided, then unsafe options can be used. */
			checksum?: Checksum;

			/** Use this flag to enable unsafe options without providing a checksum. */
			unsafe?: boolean;

			/** Whether to enable network access. Because this is an unsafe option, you must either provide a checksum for the command's output or set `unsafe` to `true`. */
			network?: boolean;
		};
	}

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
			| undefined
			| boolean
			| number
			| string
			| Uint8Array
			| Relpath
			| Subpath
			| Blob
			| Artifact
			| Placeholder
			| Template
			| Operation
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
		| undefined
		| boolean
		| number
		| string
		| Uint8Array
		| Relpath
		| Subpath
		| Blob
		| Artifact
		| Placeholder
		| Template
		| Operation
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

	// Resource.

	/** Create a resource. */
	export let resource: (arg: Resource.Arg) => Promise<Resource>;

	/** Download a resource. */
	export let download: (arg: Resource.Arg) => Promise<Artifact>;

	export class Resource {
		/** Create a symlink. */
		static new: (target: Unresolved<Resource.Arg>) => Promise<Resource>;

		/** Check if a value is a `Resource`. */
		static is: (value: unknown) => value is Resource;

		/** Expect that a value is a `Resource`. */
		static expect: (value: unknown) => Resource;

		/** Assert that a value is a `Resource`. */
		static assert: (value: unknown) => asserts value is Resource;

		/** Get this resource's hash. */
		hash(): Operation.Hash;

		/** Get this resource's URL. */
		url(): string;

		/** Get whether this resource should be unpacked. */
		unpack(): boolean;

		/** Get this resource's checksum. */
		checksum(): Checksum | undefined;

		/** Get whether this resource is unsafe. */
		unsafe(): boolean;

		/** Download this resource. */
		download(): Promise<Artifact>;
	}

	export namespace Resource {
		export type Arg = {
			/** The resource's URL. */
			url: string;

			/** The format to unpack the download with. */
			unpack?: UnpackFormat;

			/** The checksum to verify the resource. */
			checksum?: Checksum;

			/** Whether the resource should be downloaded without verifying its checksum. */
			unsafe?: boolean;
		};

		export type UnpackFormat =
			| ".tar"
			| ".tar.bz2"
			| ".tar.gz"
			| ".tar.lz"
			| ".tar.xz"
			| ".tar.zstd"
			| ".zip";
	}

	// Symlink.

	/** Create a symlink. */
	export let symlink: (target: Unresolved<Symlink.Arg>) => Promise<Symlink>;

	export class Symlink {
		/** Create a symlink. */
		static new: (target: Unresolved<Symlink.Arg>) => Promise<Symlink>;

		/** Check if a value is a `Symlink`. */
		static is: (value: unknown) => value is Symlink;

		/** Expect that a value is a `Symlink`. */
		static expect: (value: unknown) => Symlink;

		/** Assert that a value is a `Symlink`. */
		static assert: (value: unknown) => asserts value is Symlink;

		/* Get this symlink's hash. */
		hash(): Artifact.Hash;

		/** Get this symlink's target. */
		target(): Template;

		/** Resolve this symlink to the directory or file it refers to, or return undefined if none is found. */
		resolve(): Promise<Directory | File | undefined>;
	}

	export namespace Symlink {
		type Arg = Relpath.Arg | Artifact | Template | Symlink | ArgObject;

		type ArgObject = {
			artifact?: Artifact;
			path?: Subpath.Arg;
		};
	}

	// System.

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

		export type Arg = System | ArgObject;

		export type ArgObject = {
			arch: System.Arch;
			os: System.Os;
		};

		/** Create a system. */
		let new_: (arg: System.Arg) => System;
		export { new_ as new };

		/** Check if a value is a `System`. */
		export let is: (value: unknown) => value is System;

		/** Expect that a value is a `System`. */
		export let expect: (value: unknown) => System;

		/** Assert that a value is a `System`. */
		export let assert: (value: unknown) => asserts value is System;

		/** Get a system's arch. */
		export let arch: (value: System) => Arch;

		/** Get a system's OS. */
		export let os: (value: System) => Os;
	}

	// Template.

	/** Create a template. */
	export let template: (
		...args: Array<Unresolved<Template.Arg>>
	) => Promise<Template>;

	export class Template {
		static new(...args: Array<Unresolved<Template.Arg>>): Promise<Template>;

		/** Check if a value is a `Template`. */
		static is: (value: unknown) => value is Template;

		/** Expect that a value is a `Template`. */
		static expect: (value: unknown) => Template;

		/** Assert that a value is a `Template`. */
		static assert: (value: unknown) => asserts value is Template;

		/** Join an array of templates with a separator. */
		static join(
			separator: Template.Arg,
			...args: Array<Unresolved<Template.Arg>>
		): Promise<Template>;

		/** Get this template's components. */
		components(): Array<Template.Component>;
	}

	export namespace Template {
		export type Arg =
			| undefined
			| Template.Component
			| Relpath
			| Subpath
			| Template
			| Array<Arg>;

		export namespace Arg {
			/** Check if a value is a `Template.Arg`. */
			export let is: (value: unknown) => value is Template.Arg;

			/** Expect that a value is a `Template.Arg`. */
			export let expect: (value: unknown) => Template.Arg;

			/** Assert that a value is a `Template.Arg`. */
			export let assert: (value: unknown) => asserts value is Template.Arg;
		}

		export type Component = string | Artifact | Placeholder;

		export namespace Component {
			/** Check if a value is a `Template.Component`. */
			export let is: (value: unknown) => value is Template.Component;

			/** Expect that a value is a `Template.Component`. */
			export let expect: (value: unknown) => Template.Component;

			/** Assert that a value is a `Template.Component`. */
			export let assert: (
				value: unknown,
			) => asserts value is Template.Component;
		}
	}

	// Value.

	/** A `Value` is the union of all types that can be used as arguments or return values of Tangram functions. */
	export type Value =
		| undefined
		| boolean
		| number
		| string
		| Uint8Array
		| Relpath
		| Subpath
		| Blob
		| Artifact
		| Placeholder
		| Template
		| Operation
		| Array<Value>
		| { [key: string]: Value };

	export namespace Value {
		/** Check if a value is a `Value`. */
		export let is: (value: unknown) => value is Value;

		/** Expect that a value is a `Value`. */
		export let expect: (value: unknown) => Value;

		/** Assert that a value is a `Value`. */
		export let assert: (value: unknown) => asserts value is Value;
	}
}

/**
 * Create a Tangram template with a JavaScript tagged template.
 */
declare let t: (
	strings: TemplateStringsArray,
	...placeholders: Array<tg.Unresolved<tg.Template.Arg>>
) => Promise<tg.Template>;

declare let console: {
	/** Write to the log. */
	log: (...args: Array<unknown>) => void;
};
