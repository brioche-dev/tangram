declare let console: {
	log: (...args: Array<unknown>) => void;
};

declare var t: (
	strings: TemplateStringsArray,
	...placeholders: Array<
		tg.MaybePromise<tg.Template | tg.MaybeArray<tg.TemplateComponent>>
	>
) => Promise<tg.Template>;

declare namespace tg {
	// Artifact.

	export type Artifact = Directory | File | Symlink | Dependency;

	// Blob.

	type BlobLike = string | Uint8Array | Blob;

	export let blob: (blob: MaybePromise<BlobLike>) => Promise<Blob>;

	export class Blob {
		bytes: Uint8Array;
		toString(): string;
	}

	// Checksum.

	export type Checksum = {
		algorithm: ChecksumAlgorithm;
		value: string;
	};

	export type ChecksumAlgorithm = "sha256";

	// Dependency.

	type DependencyArgs = {
		artifact: MaybePromise<Artifact>;
		path?: string | null | undefined;
	};

	export let dependency: (args: DependencyArgs) => Promise<Dependency>;

	export class Dependency {
		getArtifact(): Promise<Artifact>;
		path(): Path | null | undefined;
	}

	// Directory.

	type DirectoryObject = {
		[key: string]: MaybePromise<
			undefined | null | BlobLike | Artifact | DirectoryObject
		>;
	};

	type DirectoryArg = MaybePromise<
		undefined | null | Directory | DirectoryObject
	>;

	/** Create a directory. */
	export let directory: (...args: Array<DirectoryArg>) => Promise<Directory>;

	export class Directory {
		tryGet(name: PathLike): Promise<Artifact | null>;
		get(name: PathLike): Promise<Artifact>;
		getEntries(): Promise<Record<string, Artifact>>;
		[Symbol.asyncIterator](): AsyncIterator<[string, Artifact]>;
	}

	// Download.

	type DownloadArgs = {
		url: string;
		unpack?: boolean | null | undefined;
		checksum?: Checksum | null | undefined;
		unsafe?: boolean | null | undefined;
	};

	export let download: (args: DownloadArgs) => Promise<Artifact>;

	// File.

	type FileOptions = {
		executable?: boolean;
	};

	export let file: (blobLike: BlobLike, options?: FileOptions) => Promise<File>;

	export class File {
		getBlob(): Promise<Blob>;
		executable(): boolean;
	}

	// Package.

	export let currentPackage: () => Promise<Package>;

	export class Package {
		getSource(): Promise<Artifact>;
	}

	// Path.

	export type PathLike = string | Array<PathComponent> | Path;

	export type PathComponentType =
		| "root_dir"
		| "current_dir"
		| "parent_dir"
		| "normal";

	export type PathComponent =
		| { type: "root_dir" }
		| { type: "current_dir" }
		| { type: "parent_dir" }
		| { type: "normal"; value: string };

	export let path: (path: PathLike) => Path;

	export class Path {
		components(): Array<PathComponent>;
		parent(): Path | undefined;
		join(other: PathLike): Path;
		normalize(): Path;
		toString(): string;
	}

	// Placeholder.

	export let placeholder: (name: string) => Placeholder;

	export class Placeholder {
		name: string;
	}

	// Process.

	type ProcessArgs = {
		system: System;
		env?: Record<string, TemplateLike> | null | undefined;
		command: TemplateLike;
		args?: Array<TemplateLike> | null | undefined;
		unsafe?: boolean | null | undefined;
	};

	export let process: (args: Unresolved<ProcessArgs>) => Promise<Artifact>;

	export let output: tg.Placeholder;

	// Resolve.

	export type Unresolved<T extends Value> = T extends
		| undefined
		| null
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

	export type Resolved<T extends Unresolved<Value>> = T extends
		| undefined
		| null
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

	export let resolve: <T extends Unresolved<Value>>(
		value: T,
	) => Promise<Resolved<T>>;

	// Symlink.

	export let symlink: (target: string) => Symlink;

	export class Symlink {
		target: string;
	}

	// System.

	export type System =
		| "amd64_linux"
		| "arm64_linux"
		| "amd64_macos"
		| "arm64_macos";

	export type Arch = "amd64" | "arm64";

	export type Os = "linux" | "macos";

	// Target.

	type TargetArgs = {
		package: Package;
		name: string;
		args?: Array<Value> | null | undefined;
	};

	export let target: <T extends Value>(args: TargetArgs) => Promise<T>;

	export let createTarget: <A extends Value, R extends Value>(
		f: (args: A) => MaybePromise<R>,
	) => (args: Unresolved<A>) => MaybePromise<R>;

	// Template.

	export type TemplateComponent = string | Artifact | Placeholder;

	export type TemplateLike = Template | MaybeArray<TemplateComponent>;

	export let template: (
		components: MaybeArray<
			MaybePromise<Template | MaybeArray<TemplateComponent>>
		>,
	) => Promise<Template>;

	export class Template {
		components: Array<TemplateComponent>;
	}

	// Util.

	export type MaybeArray<T> = T | Array<T>;

	export type MaybePromise<T> = T | PromiseLike<T>;

	// Value.

	export type Value =
		| (undefined | null)
		| boolean
		| number
		| string
		| Artifact
		| Placeholder
		| Template
		| Array<Value>
		| { [key: string]: Value };
}
