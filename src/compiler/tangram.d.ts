// declare var tg: typeof import("tangram-internal://core/mod.ts");

declare function syscall(syscall: "print", value: string): void;

declare function syscall(
	syscall: "add_blob",
	blob: Uint8Array,
): Promise<syscall.BlobHash>;

declare function syscall(
	syscall: "get_blob",
	hash: syscall.BlobHash,
): Promise<Uint8Array>;

declare function syscall(
	syscall: "add_artifact",
	artifact: syscall.Artifact,
): Promise<syscall.ArtifactHash>;

declare function syscall(
	syscall: "get_artifact",
	hash: syscall.ArtifactHash,
): Promise<syscall.Artifact>;

declare function syscall(
	syscall: "add_package",
	package: syscall.Package,
): Promise<syscall.PackageHash>;

declare function syscall(
	syscall: "get_package",
	hash: syscall.PackageHash,
): Promise<syscall.Package>;

declare function syscall(
	syscall: "run",
	operation: syscall.Operation,
): Promise<syscall.Value>;

declare function syscall(
	syscall: "get_current_package_hash",
): syscall.PackageHash;

declare function syscall(syscall: "get_target_name"): string;

declare namespace syscall {
	export type BlobHash = string;

	export type ArtifactHash = string;

	export type Artifact =
		| { type: "directory"; value: Directory }
		| { type: "file"; value: File }
		| { type: "symlink"; value: Symlink }
		| { type: "dependency"; value: Dependency };

	export type Directory = {
		entries: { [key: string]: ArtifactHash };
	};

	export type File = {
		blob: BlobHash;
		executable: boolean;
	};

	export type Symlink = {
		target: string;
	};

	export type Dependency = {
		artifact: ArtifactHash;
		path: string | null | undefined;
	};

	export type ValueType =
		| "null"
		| "bool"
		| "number"
		| "string"
		| "artifact"
		| "placeholder"
		| "template"
		| "array"
		| "map";

	export type Value =
		| { type: "null"; value: null | undefined }
		| { type: "bool"; value: boolean }
		| { type: "number"; value: number }
		| { type: "string"; value: string }
		| { type: "artifact"; value: ArtifactHash }
		| { type: "placeholder"; value: Placeholder }
		| { type: "template"; value: Template }
		| { type: "array"; value: Array<Value> }
		| { type: "map"; value: Record<string, Value> };

	export type Placeholder = {
		name: string;
	};

	export type Template = {
		components: Array<TemplateComponent>;
	};

	export type TemplateComponentType =
		| "string"
		| "artifact"
		| "placeholder"
		| "template";

	export type TemplateComponent =
		| { type: "string"; value: string }
		| { type: "artifact"; value: ArtifactHash }
		| { type: "placeholder"; value: Placeholder };

	export type OperationType = "download" | "process" | "target";

	export type Operation =
		| { type: "download"; value: Download }
		| { type: "process"; value: Process }
		| { type: "target"; value: Target };

	export type Download = {
		url: string;
		unpack: boolean | null | undefined;
		checksum: Checksum | null | undefined;
		unsafe: boolean | null | undefined;
	};

	export type Checksum = {
		algorithm: ChecksumAlgorithm;
		value: string;
	};

	export type ChecksumAlgorithm = "sha256";

	export type Process = {
		system: System;
		env: Record<string, Template> | null | undefined;
		command: Template;
		args: Array<Template> | null | undefined;
		unsafe: boolean | null | undefined;
	};

	export type System =
		| "amd64_linux"
		| "arm64_linux"
		| "amd64_macos"
		| "arm64_macos";

	export type Target = {
		package: PackageHash;
		name: string;
		args: Array<Value> | null | undefined;
	};

	export type PackageHash = string;

	export type Package = {
		source: ArtifactHash;
		dependencies: { [key: string]: PackageHash };
	};
}
