export type ArtifactHash = string;

export type Artifact =
	| { kind: "directory"; value: Directory }
	| { kind: "file"; value: File }
	| { kind: "symlink"; value: Symlink };

export type BlobHash = string;

export type Blob = Uint8Array;

export type Directory = {
	entries: Record<string, ArtifactHash>;
};

export type File = {
	blobHash: BlobHash;
	executable: boolean;
	references: Array<ArtifactHash>;
};

export type Symlink = {
	target: Template;
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
	checksum: Checksum | nullish;
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
	checksum: Checksum | nullish;
	unsafe: boolean;
	network: boolean;
	hostPaths: Array<string>;
};

export type System =
	| "amd64_linux"
	| "arm64_linux"
	| "amd64_macos"
	| "arm64_macos";

export type Call = {
	function: Function;
	context: Record<string, Value>;
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

export type Caller = {
	moduleIdentifier: unknown;
	position: Position;
	packageInstanceHash: PackageInstanceHash;
	line: string;
};

export type Position = {
	line: number;
	character: number;
};

declare global {
	function syscall(syscall: "log", value: string): void;

	function syscall(syscall: "caller"): Caller;

	function syscall(
		syscall: "include",
		caller: Caller,
		path: string,
	): Promise<Artifact>;

	function syscall(
		syscall: "checksum",
		algorithm: ChecksumAlgorithm,
		bytes: Uint8Array | string,
	): Checksum;

	function syscall(syscall: "encode_utf8", string: string): Uint8Array;

	function syscall(syscall: "decode_utf8", bytes: Uint8Array): string;

	function syscall(syscall: "add_blob", blob: Uint8Array): Promise<BlobHash>;

	function syscall(syscall: "get_blob", hash: BlobHash): Promise<Uint8Array>;

	function syscall(
		syscall: "add_artifact",
		artifact: Artifact,
	): Promise<ArtifactHash>;

	function syscall(
		syscall: "get_artifact",
		hash: ArtifactHash,
	): Promise<Artifact>;

	function syscall(
		syscall: "add_package_instance",
		packageInstance: PackageInstance,
	): Promise<PackageInstanceHash>;

	function syscall(
		syscall: "get_package_instance",
		hash: PackageInstanceHash,
	): Promise<PackageInstance>;

	function syscall(
		syscall: "add_operation",
		operation: Operation,
	): Promise<OperationHash>;

	function syscall(
		syscall: "get_operation",
		hash: OperationHash,
	): Promise<Operation>;

	function syscall(
		syscall: "run_operation",
		hash: OperationHash,
	): Promise<Value>;

	function syscall(
		syscall: "bundle",
		hash: ArtifactHash,
	): Promise<ArtifactHash>;
}

export let log = (value: string) => {
	try {
		return syscall("log", value);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let caller = (): Caller => {
	try {
		return syscall("caller");
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let include = async (
	caller: Caller,
	path: string,
): Promise<Artifact> => {
	try {
		return await syscall("include", caller, path);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let checksum = (
	algorithm: ChecksumAlgorithm,
	bytes: Uint8Array | string,
): Checksum => {
	try {
		return syscall("checksum", algorithm, bytes);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let encodeUtf8 = (string: string): Uint8Array => {
	try {
		return syscall("encode_utf8", string);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let decodeUtf8 = (bytes: Uint8Array): string => {
	try {
		return syscall("decode_utf8", bytes);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let addBlob = async (blob: Uint8Array): Promise<BlobHash> => {
	try {
		return await syscall("add_blob", blob);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let getBlob = async (hash: BlobHash): Promise<Uint8Array> => {
	try {
		return await syscall("get_blob", hash);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let addArtifact = async (artifact: Artifact): Promise<ArtifactHash> => {
	try {
		return await syscall("add_artifact", artifact);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let getArtifact = async (hash: ArtifactHash): Promise<Artifact> => {
	try {
		return await syscall("get_artifact", hash);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let addPackageInstance = async (
	packageInstance: PackageInstance,
): Promise<PackageInstanceHash> => {
	try {
		return await syscall("add_package_instance", packageInstance);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let getPackageInstance = async (
	hash: PackageInstanceHash,
): Promise<PackageInstance> => {
	try {
		return await syscall("get_package_instance", hash);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let addOperation = async (
	operation: Operation,
): Promise<OperationHash> => {
	try {
		return await syscall("add_operation", operation);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let getOperation = async (hash: OperationHash): Promise<Operation> => {
	try {
		return await syscall("get_operation", hash);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let runOperation = async (hash: OperationHash): Promise<Value> => {
	try {
		return await syscall("run_operation", hash);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let bundle = async (hash: ArtifactHash): Promise<ArtifactHash> => {
	try {
		return await syscall("bundle", hash);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};
