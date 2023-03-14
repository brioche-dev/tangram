declare function syscall(syscall: "log", value: string): void;

declare function syscall(
	syscall: "checksum",
	algorithm: syscall.ChecksumAlgorithm,
	bytes: Uint8Array | string,
): syscall.Checksum;

declare function syscall(syscall: "encode_utf8", string: string): Uint8Array;

declare function syscall(syscall: "decode_utf8", bytes: Uint8Array): string;

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
	syscall: "get_artifact_hash",
	hash: syscall.Artifact,
): syscall.ArtifactHash;

declare function syscall(
	syscall: "add_package_instance",
	packageInstance: syscall.PackageInstance,
): Promise<syscall.PackageInstanceHash>;

declare function syscall(
	syscall: "get_package_instance",
	hash: syscall.PackageInstanceHash,
): Promise<syscall.PackageInstance>;

declare function syscall(
	syscall: "add_operation",
	operation: syscall.Operation,
): Promise<syscall.OperationHash>;

declare function syscall(
	syscall: "get_operation",
	hash: syscall.OperationHash,
): Promise<syscall.Operation>;

declare function syscall(
	syscall: "run",
	hash: syscall.OperationHash,
): Promise<syscall.Value>;

declare function syscall(
	syscall: "get_current_package_instance_hash",
): syscall.PackageInstanceHash;

declare function syscall(syscall: "get_current_export_name"): string;

declare namespace syscall {
	export type ArtifactHash = string;

	export type Artifact =
		| { kind: "directory"; value: Directory }
		| { kind: "file"; value: File }
		| { kind: "symlink"; value: Symlink }
		| { kind: "reference"; value: Reference };

	export type BlobHash = string;

	export type Blob = Uint8Array;

	export type Directory = {
		entries: Record<string, ArtifactHash>;
	};

	export type File = {
		blobHash: BlobHash;
		executable: boolean;
	};

	export type Symlink = {
		target: string;
	};

	export type Reference = {
		artifactHash: ArtifactHash;
		path: string | nullish;
	};

	export type Value =
		| { kind: "null"; value: nullish }
		| { kind: "bool"; value: boolean }
		| { kind: "number"; value: number }
		| { kind: "string"; value: string }
		| { kind: "artifact"; value: ArtifactHash }
		| { kind: "placeholder"; value: Placeholder }
		| { kind: "template"; value: Template }
		| { kind: "array"; value: Array<Value> }
		| { kind: "map"; value: Record<string, Value> };

	export type Placeholder = {
		name: string;
	};

	export type Template = {
		components: Array<TemplateComponent>;
	};

	export type TemplateComponent =
		| { kind: "string"; value: string }
		| { kind: "artifact"; value: ArtifactHash }
		| { kind: "placeholder"; value: Placeholder };

	export type OperationHash = string;

	export type Operation =
		| { kind: "call"; value: Call }
		| { kind: "download"; value: Download }
		| { kind: "process"; value: Process };

	export type Download = {
		url: string;
		unpack: boolean;
		checksum: string | nullish;
		unsafe: boolean;
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

	export type Process = {
		system: System;
		env: Record<string, Template>;
		command: Template;
		args: Array<Template>;
		unsafe: boolean;
	};

	export type System =
		| "amd64_linux"
		| "arm64_linux"
		| "amd64_macos"
		| "arm64_macos";

	export type Call = {
		function: Function;
		context: { [key: string]: Value };
		args: Array<Value>;
	};

	export type Function = {
		packageInstanceHash: PackageInstanceHash;
		name: string;
	};

	export type PackageInstanceHash = string;

	export type PackageInstance = {
		packageHash: ArtifactHash;
		dependencies: Record<string, PackageInstanceHash>;
	};

	export type Checksum = `${ChecksumAlgorithm}${":" | "-"}${string}`;

	export type ChecksumAlgorithm = "blake3" | "sha256";

	export type nullish = undefined | null;
}
