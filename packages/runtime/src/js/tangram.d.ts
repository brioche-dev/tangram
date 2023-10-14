/// <reference lib="es2023" />

declare namespace tg {
	/** An artifact. */
	export type Artifact = Directory | File | Symlink;

	export namespace Artifact {
		export type Id = string;

		/** Get an artifact with an ID. */
		export let withId: (id: Artifact.Id) => Artifact;

		/** Check if a value is an `Artifact`. */
		export let is: (value: unknown) => value is Artifact;

		/** Expect that a value is an `Artifact`. */
		export let expect: (value: unknown) => Artifact;

		/** Assert that a value is an `Artifact`. */
		export let assert: (value: unknown) => asserts value is Artifact;
	}

	export let assert: (
		condition: unknown,
		message?: string,
	) => asserts condition;

	export let unimplemented: (message?: string) => never;

	export let unreachable: (message?: string) => never;

	/** Create a blob. */
	export let blob: (...args: Array<Unresolved<Blob.Arg>>) => Promise<Blob>;

	/** Download the contents of a URL. */
	export let download: (url: string, checksum: Checksum) => Promise<Blob>;

	/** A blob. */
	export class Blob {
		/** Get a blob with an ID. */
		static withId(id: Blob.Id): Blob;

		/** Create a blob. */
		static new(...args: Array<Unresolved<Blob.Arg>>): Promise<Blob>;

		/** Download the contents of a URL. */
		static download(url: string, checksum: Checksum): Promise<Blob>;

		/** Check if a value is a `Blob`. */
		static is(value: unknown): value is Blob;

		/** Expect that a value is a `Blob`. */
		static expect(value: unknown): Blob;

		/** Assert that a value is a `Blob`. */
		static assert(value: unknown): asserts value is Blob;

		/* Get this blob's id. */
		id(): Promise<Blob.Id>;

		/** Get this blob's size. */
		size(): Promise<number>;

		/** Get this blob as a `Uint8Array`. */
		bytes(): Promise<Uint8Array>;

		/** Get this blob as a string. */
		text(): Promise<string>;

		/** Decompress this blob. */
		decompress(format: Blob.CompressionFormat): Promise<Blob>;

		/** Extract an artifact from this blob. */
		extract(format: Blob.ArchiveFormat): Promise<Artifact>;
	}

	export namespace Blob {
		export type Arg = undefined | string | Uint8Array | Blob | Array<Arg>;

		export type Id = string;

		type ArchiveFormat = ".tar" | ".zip";

		type CompressionFormat = ".bz2" | ".gz" | ".lz" | ".xz" | ".zstd";
	}

	/** Compute the checksum of the provided bytes. */
	export let checksum: (
		algorithm: Checksum.Algorithm,
		bytes: string | Uint8Array,
	) => Checksum;

	export type Checksum = string;

	export namespace Checksum {
		export type Algorithm = "blake3" | "sha256" | "sha512";

		export let new_: (
			algorithm: Algorithm,
			bytes: string | Uint8Array,
		) => Checksum;
		export { new_ as new };
	}

	/** Create a directory. */
	export let directory: (
		...args: Array<Unresolved<Directory.Arg>>
	) => Promise<Directory>;

	/** A directory. */
	export class Directory {
		/** Get a directory with an ID. */
		static withId(id: Directory.Id): Directory;

		/** Create a directory. */
		static new(...args: Array<Unresolved<Directory.Arg>>): Promise<Directory>;

		/** Check if a value is a `Directory`. */
		static is(value: unknown): value is Directory;

		/** Expect that a value is a `Directory`. */
		static expect(value: unknown): Directory;

		/** Assert that a value is a `Directory`. */
		static assert(value: unknown): asserts value is Directory;

		/* Get this directory's id. */
		id(): Promise<Directory.Id>;

		/** Get this directory's entries. */
		entries(): Promise<Record<string, Artifact>>;

		/** Get the child at the specified path. This method throws an error if the path does not exist. */
		get(arg: string): Promise<Artifact>;

		/** Try to get the child at the specified path. This method returns `undefined` if the path does not exist. */
		tryGet(arg: string): Promise<Artifact | undefined>;

		/** Bundle this directory. */
		bundle: () => Promise<Directory>;

		/** Get an async iterator of this directory's recursive entries. */
		walk(): AsyncIterableIterator<[string, Artifact]>;

		/** Get an async iterator of this directory's entries. */
		[Symbol.asyncIterator](): AsyncIterator<[string, Artifact]>;
	}

	export namespace Directory {
		export type Arg = undefined | Directory | Array<Arg> | ArgObject;

		type ArgObject = {
			[key: string]: undefined | Blob.Arg | Artifact | ArgObject;
		};

		export type Id = string;
	}

	export namespace encoding {
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
	}

	/** Create a file. */
	export let file: (...args: Array<Unresolved<File.Arg>>) => Promise<File>;

	/** A file. */
	export class File {
		/** Get a file with an ID. */
		static withId(id: File.Id): File;

		/** Create a file. */
		static new(...args: Array<Unresolved<File.Arg>>): Promise<File>;

		/** Check if a value is a `File`. */
		static is(value: unknown): value is File;

		/** Expect that a value is a `File`. */
		static expect(value: unknown): File;

		/** Assert that a value is a `File`. */
		static assert(value: unknown): asserts value is File;

		/* Get this file's id. */
		id(): Promise<File.Id>;

		/** Get this file's contents. */
		contents(): Promise<Blob>;

		/** Get the size of this file's contents. */
		size(): Promise<number>;

		/** Get this file's contents as a `Uint8Array`. */
		bytes(): Promise<Uint8Array>;

		/** Get this file's contents as a string. This method throws an error if the contents are not valid UTF-8. */
		text(): Promise<string>;

		/** Get this file's executable bit. */
		executable(): Promise<boolean>;

		/** Get this file's references. */
		references(): Promise<Array<Artifact>>;
	}

	export namespace File {
		export type Arg = Blob.Arg | File | Array<Arg> | ArgObject;

		export type ArgObject = {
			contents: Blob.Arg;
			executable?: boolean;
			references?: Array<Artifact>;
		};

		export type Id = string;
	}

	/** Include an artifact at a path relative to the module this function is called from. The path must be a string literal so that it can be statically analyzed. */
	export let include: (path: string) => Promise<Artifact>;

	/** Write to the log. */
	export let log: (...args: Array<unknown>) => void;

	/** A package. */
	export class Package {
		/** Get a package with an ID. */
		static withId(id: Package.Id): Package;

		/** Check if a value is a `Package`. */
		static is(value: unknown): value is Package;

		/** Expect that a value is a `Package`. */
		static expect(value: unknown): Package;

		/** Assert that a value is a `Package`. */
		static assert(value: unknown): asserts value is Package;

		/** Get this package's artifact. */
		artifact(): Promise<Artifact>;

		/** Get this package's dependencies. */
		dependencies(): Promise<Record<string, Package>>;
	}

	export namespace Package {
		export type Id = string;
	}

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
			| Blob
			| Directory
			| File
			| Symlink
			| Template
			| Package
			| Target
			? T
			: T extends Array<infer U extends Value>
			? Array<Unresolved<U>>
			: T extends { [key: string]: Value }
			? { [K in keyof T]: Unresolved<T[K]> }
			: never
	>;

	/**
	 * This computed type performs the inverse of `Unresolved`. It takes a type and returns the output of calling `resolve` on a value of that type. Here are some examples:
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
		| Blob
		| Directory
		| File
		| Symlink
		| Template
		| Package
		| Target
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

	/** Create a symlink. */
	export let symlink: (
		...args: Array<Unresolved<Symlink.Arg>>
	) => Promise<Symlink>;

	export class Symlink {
		/** Get a symlink with an ID. */
		static withId(id: Symlink.Id): Symlink;

		/** Create a symlink. */
		static new(...args: Array<Unresolved<Symlink.Arg>>): Promise<Symlink>;

		/** Check if a value is a `Symlink`. */
		static is(value: unknown): value is Symlink;

		/** Expect that a value is a `Symlink`. */
		static expect(value: unknown): Symlink;

		/** Assert that a value is a `Symlink`. */
		static assert(value: unknown): asserts value is Symlink;

		/* Get this symlink's id. */
		id(): Promise<Symlink.Id>;

		/** Get this symlink's target. */
		target(): Promise<Template>;

		/** Resolve this symlink to the directory or file it refers to, or return undefined if none is found. */
		resolve(): Promise<Directory | File | undefined>;
	}

	export namespace Symlink {
		export type Arg =
			| undefined
			| string
			| Artifact
			| Template
			| Symlink
			| ArgObject;

		type ArgObject = {
			artifact?: Artifact;
			path?: string | undefined;
		};

		export type Id = string;
	}

	/** Create a system. */
	export let system: (arg: System.Arg) => System;

	export type System =
		| "aarch64-darwin"
		| "aarch64-linux"
		| "js-js"
		| "x86_64-darwin"
		| "x86_64-linux";

	export namespace System {
		export type Arg = System | ArgObject;

		export type ArgObject = {
			arch: System.Arch;
			os: System.Os;
		};

		export type Arch = "aarch64" | "js" | "x86_64";

		export type Os = "darwin" | "js" | "linux";

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

	/** Create a target. */
	export function target<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	>(function_: (...args: A) => MaybePromise<R | void>): Target<A, R>;
	export function target<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	>(...args: Array<Unresolved<Target.Arg>>): Promise<Target<A, R>>;

	/** Create and build a target. */
	export function build<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	>(...args: Array<Unresolved<Target.Arg>>): Promise<Target<A, R>>;

	/** A target. */
	export interface Target<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	> {
		/** Build this target. */
		(...args: { [K in keyof A]: Unresolved<A[K]> }): Promise<R>;
	}

	export class Target<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	> extends globalThis.Function {
		/** Get a target with an ID. */
		static withId(id: Target.Id): Target;

		/** Create a target. */
		static new<A extends Array<Value> = Array<Value>, R extends Value = Value>(
			arg: (...args: A) => MaybePromise<R | void> | Unresolved<Target.Arg>,
		): Promise<Target<A, R>>;

		/** Check if a value is a `Target`. */
		static is(value: unknown): value is Target;

		/** Expect that a value is a `Target`. */
		static expect(value: unknown): Target;

		/** Assert that a value is a `Target`. */
		static assert(value: unknown): asserts value is Target;

		/* Get this target's id. */
		id(): Promise<Target.Id>;

		/** Get this target's package. */
		package(): Promise<string | undefined>;

		/** Get this target's host. */
		host(): Promise<System>;

		/** Get this target's executable. */
		executable(): Promise<Template>;

		/** Get this target's name. */
		name_(): Promise<string | undefined>;

		/** Get this target's environment. */
		env(): Promise<Record<string, Value>>;

		/** Get this target's arguments. */
		args(): Promise<Array<Value>>;

		/** Get this target's checksum. */
		checksum(): Promise<Checksum | undefined>;

		/** Get whether this target is unsafe. */
		unsafe(): Promise<boolean>;

		/** Build this target. */
		build(...args: { [K in keyof A]: Unresolved<A[K]> }): Promise<R>;
	}

	export namespace Target {
		export type Arg = Template | Target | Array<Arg> | ArgObject;

		type ArgObject = {
			/** The system to build the target on. */
			host?: System;

			/** The target's executable. */
			executable?: Template.Arg;

			/** The target's package. */
			package?: Package | undefined;

			/** The target's name. */
			name?: string | undefined;

			/** The target's environment variables. */
			env?: Record<string, Value>;

			/** The target's command line arguments. */
			args?: Array<Value>;

			/** If a checksum of the target's output is provided, then the target will have access to the network. */
			checksum?: Checksum | undefined;

			/** If the target is marked as unsafe, then it will have access to the network even if a checksum is not provided. */
			unsafe?: boolean;
		};

		export type Id = string;
	}

	/** Create a template. */
	export let template: (
		...args: Array<Unresolved<Template.Arg>>
	) => Promise<Template>;

	export class Template {
		static new(...args: Array<Unresolved<Template.Arg>>): Promise<Template>;

		/** Check if a value is a `Template`. */
		static is(value: unknown): value is Template;

		/** Expect that a value is a `Template`. */
		static expect(value: unknown): Template;

		/** Assert that a value is a `Template`. */
		static assert(value: unknown): asserts value is Template;

		/** Join an array of templates with a separator. */
		static join(
			separator: Template.Arg,
			...args: Array<Unresolved<Template.Arg>>
		): Promise<Template>;

		/** Get this template's components. */
		get components(): Array<Template.Component>;
	}

	export namespace Template {
		export type Arg = undefined | Template.Component | Template | Array<Arg>;

		export type Component = string | Artifact;

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

	/** `Value` is the union of all types that can be used as the input or output of Tangram targets. */
	export type Value =
		| undefined
		| boolean
		| number
		| string
		| Uint8Array
		| Blob
		| Directory
		| File
		| Symlink
		| Template
		| Package
		| Target
		| Array<Value>
		| { [key: string]: Value };

	export namespace Value {
		export type Id = string;

		/** Get a value with an ID. */
		export let withId: (id: Value.Id) => Value;

		/** Check if a value is `Value`. */
		export let is: (value: unknown) => value is Value;

		/** Expect that a value is `Value`. */
		export let expect: (value: unknown) => Value;

		/** Assert that a value is `Value`. */
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
