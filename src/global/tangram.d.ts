/// <reference lib="es2023" />

declare namespace tg {
	export let assert: (
		condition: unknown,
		message?: string,
	) => asserts condition;

	export let unimplemented: (message?: string) => never;

	export let unreachable: (message?: string) => never;

	/** An artifact. */
	export type Artifact = Directory | File | Symlink;

	export namespace Artifact {
		/** Get an artifact with an ID. */
		export let withId: (id: Id) => Promise<Artifact>;

		/** Check if a value is an `Artifact`. */
		export let is: (value: unknown) => value is Artifact;

		/** Expect that a value is an `Artifact`. */
		export let expect: (value: unknown) => Artifact;

		/** Assert that a value is an `Artifact`. */
		export let assert: (value: unknown) => asserts value is Artifact;
	}

	/** Create a blob. */
	export let blob: (...args: Array<Unresolved<Blob.Arg>>) => Promise<Blob>;

	export class Blob {
		/** Get a blob with an ID. */
		static withId(id: Id): Promise<Blob>;

		/** Create a blob. */
		static new(...args: Array<Unresolved<Blob.Arg>>): Promise<Blob>;

		/** Check if a value is a `Blob`. */
		static is(value: unknown): value is Blob;

		/** Expect that a value is a `Blob`. */
		static expect(value: unknown): Blob;

		/** Assert that a value is a `Blob`. */
		static assert(value: unknown): asserts value is Blob;

		/* Get this blob's id. */
		id(): Promise<Id>;

		/** Get this blob's size. */
		size(): Promise<number>;

		/** Get this blob as a `Uint8Array`. */
		bytes(): Promise<Uint8Array>;

		/** Get this blob as a string. */
		text(): Promise<string>;
	}

	export namespace Blob {
		export type Arg = undefined | string | Uint8Array | Blob | Array<Arg>;
	}

	export type Build = Resource | Target | Task;

	export namespace Build {
		/** Get a build with an ID. */
		export let withId: (id: Id) => Promise<Build>;

		/** Check if a value is a `Build`. */
		export let is: (value: unknown) => value is Build;

		/** Expect that a value is a `Build`. */
		export let expect: (value: unknown) => Build;

		/** Assert that a value is a `Build`. */
		export let assert: (value: unknown) => asserts value is Build;
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
		static withId(id: Id): Promise<Directory>;

		/** Create a directory. */
		static new(...args: Array<Unresolved<Directory.Arg>>): Promise<Directory>;

		/** Check if a value is a `Directory`. */
		static is(value: unknown): value is Directory;

		/** Expect that a value is a `Directory`. */
		static expect(value: unknown): Directory;

		/** Assert that a value is a `Directory`. */
		static assert(value: unknown): asserts value is Directory;

		/* Get this directory's id. */
		id(): Promise<Id>;

		/** Get the child at the specified path. This method throws an error if the path does not exist. */
		get(arg: Subpath.Arg): Promise<Artifact>;

		/** Try to get the child at the specified path. This method returns `undefined` if the path does not exist. */
		tryGet(arg: Subpath.Arg): Promise<Artifact | undefined>;

		/** Get this directory's entries. */
		entries(): Promise<Record<string, Artifact>>;

		/** Bundle this directory. */
		bundle: () => Promise<Directory>;

		/** Get an iterator of this directory's recursive entries. */
		walk(): AsyncIterableIterator<[Subpath, Artifact]>;

		/** Get an async iterator of this directory's entries. */
		[Symbol.asyncIterator](): AsyncIterator<[string, Artifact]>;
	}

	export namespace Directory {
		type Arg = undefined | Directory | Array<Arg> | ArgObject;

		type ArgObject = { [key: string]: ArgObjectValue };

		type ArgObjectValue = undefined | Blob.Arg | Artifact | ArgObject;
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

	export namespace env {
		export let get: () => Record<string, Any>;
	}

	/** Create a file. */
	export let file: (...args: Array<Unresolved<File.Arg>>) => Promise<File>;

	/** A file. */
	export class File {
		/** Get a file with an ID. */
		static withId(id: Id): Promise<File>;

		/** Create a file. */
		static new(...args: Array<Unresolved<File.Arg>>): Promise<File>;

		/** Check if a value is a `File`. */
		static is(value: unknown): value is File;

		/** Expect that a value is a `File`. */
		static expect(value: unknown): File;

		/** Assert that a value is a `File`. */
		static assert(value: unknown): asserts value is File;

		/* Get this file's id. */
		id(): Promise<Id>;

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
	}

	export type Id = string;

	/** Include an artifact at a path relative to the module this function is called from. The path must be a string literal so that it can be statically analyzed. */
	export let include: (path: string) => Promise<Artifact>;

	/** Write to the log. */
	export let log: (...args: Array<unknown>) => void;

	export class Package {
		/** Get this package's artifact. */
		artifact(): Promise<Artifact>;

		/** Get this package's dependencies. */
		dependencies(): Promise<Record<string, Package>>;
	}

	/** Create a relative path. */
	export let relpath: (...args: Array<Relpath.Arg>) => Relpath;

	/** A relative path. */
	export class Relpath {
		/** Create a new relpath. */
		static new(...args: Array<Relpath.Arg>): Relpath;

		/** Check if a value is a `Relpath`. */
		static is(value: unknown): value is Relpath;

		/** Expect that a value is a `Relpath`. */
		static expect(value: unknown): Relpath;

		/** Assert that a value is a `Relpath`. */
		static assert(value: unknown): asserts value is Relpath;

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
		static new(...args: Array<Subpath.Arg>): Subpath;

		/** Check if a value is a `Subpath`. */
		static is(value: unknown): value is Subpath;

		/** Expect that a value is a `Subpath`. */
		static expect(value: unknown): Subpath;

		/** Assert that a value is a `Subpath`. */
		static assert(value: unknown): asserts value is Subpath;

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

	/** Create a placeholder. */
	export let placeholder: (name: string) => Placeholder;

	/** A placeholder. */
	export class Placeholder {
		/** Create a new placeholder. */
		static new(name: string): Placeholder;

		/** Check if a value is a `Placeholder`. */
		static is(value: unknown): value is Placeholder;

		/** Expect that a value is a `Placeholder`. */
		static expect(value: unknown): Placeholder;

		/** Assert that a value is a `Placeholder`. */
		static assert(value: unknown): asserts value is Placeholder;

		/** Get this placeholder's name. */
		name(): string;
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
	export type Unresolved<T extends Any> = MaybePromise<
		T extends
			| undefined
			| boolean
			| number
			| string
			| Uint8Array
			| Relpath
			| Subpath
			| Blob
			| Directory
			| File
			| Symlink
			| Placeholder
			| Template
			| Package
			| Resource
			| Target
			| Task
			? T
			: T extends Array<infer U extends Any>
			? Array<Unresolved<U>>
			: T extends { [key: string]: Any }
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
	export type Resolved<T extends Unresolved<Any>> = T extends
		| undefined
		| boolean
		| number
		| string
		| Uint8Array
		| Relpath
		| Subpath
		| Blob
		| Directory
		| File
		| Symlink
		| Placeholder
		| Template
		| Package
		| Resource
		| Target
		| Task
		? T
		: T extends Array<infer U extends Unresolved<Any>>
		? Array<Resolved<U>>
		: T extends { [key: string]: Unresolved<Any> }
		? { [K in keyof T]: Resolved<T[K]> }
		: T extends Promise<infer U extends Unresolved<Any>>
		? Resolved<U>
		: never;

	/** Resolve all deeply nested promises in an unresolved value. */
	export let resolve: <T extends Unresolved<Any>>(
		value: T,
	) => Promise<Resolved<T>>;

	export type MaybePromise<T> = T | Promise<T>;

	/** Create a resource. */
	export let resource: (arg: Resource.Arg) => Promise<Resource>;

	/** Download a resource. */
	export let download: (arg: Resource.Arg) => Promise<Artifact>;

	export class Resource {
		/** Get a resource with an ID. */
		static withId(id: Id): Promise<Resource>;

		/** Create a symlink. */
		static new(target: Unresolved<Resource.Arg>): Promise<Resource>;

		/** Check if a value is a `Resource`. */
		static is(value: unknown): value is Resource;

		/** Expect that a value is a `Resource`. */
		static expect(value: unknown): Resource;

		/** Assert that a value is a `Resource`. */
		static assert(value: unknown): asserts value is Resource;

		/* Get this resource's id. */
		id(): Promise<Id>;

		/** Get this resource's URL. */
		url(): Promise<string>;

		/** Get whether this resource should be unpacked. */
		unpack(): Promise<Resource.UnpackFormat | undefined>;

		/** Get this resource's checksum. */
		checksum(): Promise<Checksum | undefined>;

		/** Get whether this resource is unsafe. */
		unsafe(): Promise<boolean>;

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

	/** Create a symlink. */
	export let symlink: (
		...args: Array<Unresolved<Symlink.Arg>>
	) => Promise<Symlink>;

	export class Symlink {
		/** Get a symlink with an ID. */
		static withId(id: Id): Promise<Symlink>;

		/** Create a symlink. */
		static new(...args: Array<Unresolved<Symlink.Arg>>): Promise<Symlink>;

		/** Check if a value is a `Symlink`. */
		static is(value: unknown): value is Symlink;

		/** Expect that a value is a `Symlink`. */
		static expect(value: unknown): Symlink;

		/** Assert that a value is a `Symlink`. */
		static assert(value: unknown): asserts value is Symlink;

		/* Get this symlink's id. */
		id(): Promise<Id>;

		/** Get this symlink's target. */
		target(): Promise<Template>;

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

	/** Create a system. */
	export let system: (arg: System.Arg) => System;

	export type System =
		| "amd64_linux"
		| "arm64_linux"
		| "amd64_macos"
		| "arm64_macos";

	export namespace System {
		export type Arg = System | ArgObject;

		export type ArgObject = {
			arch: System.Arch;
			os: System.Os;
		};

		export type Arch = "amd64" | "arm64";

		export type Os = "linux" | "macos";

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
	function target<A extends Array<Any> = Array<Any>, R extends Any = Any>(
		f: (...args: A) => MaybePromise<R | void>,
	): Promise<Target<A, R>>;
	function target<A extends Array<Any> = Array<Any>, R extends Any = Any>(
		name: string,
		f: (...args: A) => MaybePromise<R | void>,
	): Promise<Target<A, R>>;

	/** Build a target. */
	export let build: <A extends Array<Any> = Array<Any>, R extends Any = Any>(
		arg: Target.Arg<A, R>,
	) => Promise<R>;

	/** A target. */
	export interface Target<
		A extends Array<Any> = Array<Any>,
		R extends Any = Any,
	> {
		/** Build this target. */
		(...args: { [K in keyof A]: Unresolved<A[K]> }): Promise<R>;
	}

	export class Target<
		A extends Array<Any> = Array<Any>,
		R extends Any = Any,
	> extends globalThis.Function {
		/** Get a target with an ID. */
		static withId(id: Id): Promise<Target>;

		/** Check if a value is a `Target`. */
		static is(value: unknown): value is Target;

		/** Expect that a value is a `Target`. */
		static expect(value: unknown): Target;

		/** Assert that a value is a `Target`. */
		static assert(value: unknown): asserts value is Target;

		/* Get this target's id. */
		id(): Promise<Id>;

		/** The path to the module in the package where the target is defined. */
		path(): Promise<Subpath>;

		/** Get the target's name. */
		name_(): Promise<string>;

		/** Get this target's env. */
		env(): Promise<Record<string, Any>>;

		/** Get this target's args. */
		args(): Promise<Array<Any>>;
	}

	export namespace Target {
		export type Arg<A extends Array<Any> = Array<Any>, R extends Any = Any> = {
			target: Target<A, R>;
			env?: Unresolved<Record<string, Any>>;
			args: Unresolved<A>;
		};
	}

	/** Create a task. */
	export let task: (arg: Unresolved<Task.Arg>) => Promise<Task>;

	/** Run a task. */
	export let run: (arg: Unresolved<Task.Arg>) => Promise<Artifact | undefined>;

	/** The output placeholder for a task. */
	export let output: Placeholder;

	/** A task. */
	export class Task {
		/** Get a task with an ID. */
		static withId(id: Id): Promise<Task>;

		/** Create a task. */
		static new(target: Unresolved<Task.Arg>): Promise<Task>;

		/** Check if a value is a `Task`. */
		static is(value: unknown): value is Task;

		/** Expect that a value is a `Task`. */
		static expect(value: unknown): Task;

		/** Assert that a value is a `Task`. */
		static assert(value: unknown): asserts value is Task;

		/* Get this task's id. */
		id(): Promise<Id>;

		/** Get this task's host. */
		host(): Promise<System>;

		/** Get this task's executable. */
		executable(): Promise<Template>;

		/** Get this task's environment variables. */
		env(): Promise<Record<string, Template>>;

		/** Get this task's command line arguments. */
		args(): Promise<Array<Template>>;

		/** Get this task's checksum. */
		checksum(): Promise<Checksum | undefined>;

		/** Get whether this task is unsafe. */
		unsafe(): Promise<boolean>;

		/** Get whether this task has the network enabled. */
		network(): Promise<boolean>;

		/** Run this task. */
		run(): Promise<Promise<Artifact | undefined>>;
	}

	export namespace Task {
		export type Arg = {
			/** The system to run the task on. */
			host: System;

			/** The task's executable. */
			executable: Template.Arg;

			/** The task's environment variables. */
			env?: Record<string, Template.Arg>;

			/** The task's command line arguments. */
			args?: Array<Template.Arg>;

			/** A checksum of the task's output. If a checksum is provided, then unsafe options can be used. */
			checksum?: Checksum;

			/** If this flag is set, then unsafe options can be used without a checksum. */
			unsafe?: boolean;

			/** If this flag is set, then the task will have access to the network. This is an unsafe option. */
			network?: boolean;
		};
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

	/** `Any` is the union of all types that can be used as the input or output of Tangram targets. */
	export type Any =
		| undefined
		| boolean
		| number
		| string
		| Uint8Array
		| Relpath
		| Subpath
		| Blob
		| Directory
		| File
		| Symlink
		| Placeholder
		| Template
		| Package
		| Resource
		| Target
		| Task
		| Array<Any>
		| { [key: string]: Any };

	export namespace Any {
		/** Get a value with an ID. */
		export let withId: (id: Id) => Promise<Any>;

		/** Check if a value is `Any`. */
		export let is: (value: unknown) => value is Any;

		/** Expect that a value is `Any`. */
		export let expect: (value: unknown) => Any;

		/** Assert that a value is `Any`. */
		export let assert: (value: unknown) => asserts value is Any;
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
