export type ArtifactHash = string;

export type Artifact =
	| { kind: "directory"; value: Directory }
	| { kind: "file"; value: File }
	| { kind: "symlink"; value: Symlink };

export type BlobHash = string;

export type Blob = {
	hash: BlobHash;
};

export type Directory = {
	hash: ArtifactHash;
	entries: Record<string, ArtifactHash>;
};

export type File = {
	hash: ArtifactHash;
	blob: Blob;
	executable: boolean;
	references: Array<ArtifactHash>;
};

export type Symlink = {
	hash: ArtifactHash;
	target: Template;
};

export type Value =
	| { kind: "null"; value: nullish }
	| { kind: "bool"; value: boolean }
	| { kind: "number"; value: number }
	| { kind: "string"; value: string }
	| { kind: "bytes"; value: Uint8Array }
	| { kind: "path"; value: Path }
	| { kind: "blob"; value: Blob }
	| { kind: "artifact"; value: Artifact }
	| { kind: "placeholder"; value: Placeholder }
	| { kind: "template"; value: Template }
	| { kind: "array"; value: Array<Value> }
	| { kind: "object"; value: Record<string, Value> };

export type Path = string;

export type Placeholder = {
	name: string;
};

export type Template = {
	components: Array<TemplateComponent>;
};

export type TemplateComponent =
	| { kind: "string"; value: string }
	| { kind: "artifact"; value: Artifact }
	| { kind: "placeholder"; value: Placeholder };

export type OperationHash = string;

export type Operation =
	| { kind: "call"; value: Call }
	| { kind: "download"; value: Download }
	| { kind: "process"; value: Process };

export type Download = {
	hash: OperationHash;
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
	hash: OperationHash;
	system: System;
	executable: Template;
	env: Record<string, Template>;
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
	hash: OperationHash;
	function: Function;
	env: Record<string, Value>;
	args: Array<Value>;
};

export type Function = {
	packageInstanceHash: PackageInstanceHash;
	name: string;
};

export type PackageInstanceHash = string;

export type PackageInstance = {
	hash: PackageInstanceHash;
	packageHash: ArtifactHash;
	dependencies: Record<string, PackageInstanceHash>;
};

export type Checksum = `${ChecksumAlgorithm}${":" | "-"}${string}`;

export type ChecksumAlgorithm = "blake3" | "sha256";

export type nullish = undefined | null;

export type StackFrame = {
	module: Module;
	position: Position;
	line: string;
};

export type Module =
	| { kind: "library"; value: LibraryModule }
	| { kind: "document"; value: DocumentModule }
	| { kind: "normal"; value: NormalModule };

export type LibraryModule = {
	modulePath: string;
};

export type DocumentModule = {
	packagePath: string;
	modulePath: string;
};

export type NormalModule = {
	packageInstanceHash: string;
	modulePath: string;
};

export type Position = {
	line: number;
	character: number;
};

declare global {
	function syscall(
		syscall: "artifact_bundle",
		artifact: Artifact,
	): Promise<Artifact>;

	function syscall(
		syscall: "artifact_get",
		hash: ArtifactHash,
	): Promise<Artifact>;
}

export let artifact = {
	bundle: async (artifact: Artifact): Promise<Artifact> => {
		try {
			return await syscall("artifact_bundle", artifact);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	get: async (hash: ArtifactHash): Promise<Artifact> => {
		try {
			return await syscall("artifact_get", hash);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	/** Decode a base64 string to bytes. */
	function syscall(syscall: "base64_decode", value: string): Uint8Array;

	/** Encode bytes to a base64 string. */
	function syscall(syscall: "base64_encode", value: Uint8Array): string;
}

export let base64 = {
	decode: (value: string): Uint8Array => {
		try {
			return syscall("base64_decode", value);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	encode: (value: Uint8Array): string => {
		try {
			return syscall("base64_encode", value);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	function syscall(syscall: "blob_bytes", blob: Blob): Promise<Uint8Array>;

	function syscall(
		syscall: "blob_new",
		bytes: Uint8Array | string,
	): Promise<Blob>;

	function syscall(syscall: "blob_text", blob: Blob): Promise<string>;
}

export let blob = {
	bytes: async (blob: Blob): Promise<Uint8Array> => {
		try {
			return await syscall("blob_bytes", blob);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	new: async (bytes: Uint8Array | string): Promise<Blob> => {
		try {
			return await syscall("blob_new", bytes);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	text: async (blob: Blob): Promise<string> => {
		try {
			return await syscall("blob_text", blob);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	function syscall(
		syscall: "call_new",
		function_: Function,
		env: Record<string, Value>,
		args: Array<Value>,
	): Promise<Call>;
}

export let call = {
	new: async (
		function_: Function,
		env: Record<string, Value>,
		args: Array<Value>,
	): Promise<Call> => {
		try {
			return await syscall("call_new", function_, env, args);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	function syscall(
		syscall: "checksum",
		algorithm: ChecksumAlgorithm,
		bytes: Uint8Array,
	): Checksum;
}

export let checksum = (
	algorithm: ChecksumAlgorithm,
	bytes: Uint8Array,
): Checksum => {
	try {
		return syscall("checksum", algorithm, bytes);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

declare global {
	function syscall(
		syscall: "download_new",
		url: string,
		unpack: boolean,
		checksum: Checksum | nullish,
		unsafe: boolean,
	): Promise<Download>;
}

export let download = {
	new: async (
		url: string,
		unpack: boolean,
		checksum: Checksum | nullish,
		unsafe: boolean,
	): Promise<Download> => {
		try {
			return await syscall("download_new", url, unpack, checksum, unsafe);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	function syscall(
		syscall: "directory_new",
		entries: Map<string, Artifact>,
	): Promise<Directory>;
}

export let directory = {
	new: async (entries: Map<string, Artifact>): Promise<Directory> => {
		try {
			return await syscall("directory_new", entries);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	function syscall(
		syscall: "file_new",
		blob: Blob,
		executable: boolean,
		references: Array<Artifact>,
	): Promise<File>;
}

export let file = {
	new: async (
		blob: Blob,
		executable: boolean,
		references: Array<Artifact>,
	): Promise<File> => {
		try {
			return await syscall("file_new", blob, executable, references);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	/** Decode a hex string to bytes. */
	function syscall(syscall: "hex_decode", value: string): Uint8Array;

	/** Encode bytes to a hex string. */
	function syscall(syscall: "hex_encode", value: Uint8Array): string;
}

export let hex = {
	decode: (value: string): Uint8Array => {
		try {
			return syscall("hex_decode", value);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	encode: (value: Uint8Array): string => {
		try {
			return syscall("hex_encode", value);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	function syscall(
		syscall: "include",
		stackFrame: StackFrame,
		path: string,
	): Promise<Artifact>;
}

export let include = async (
	stackFrame: StackFrame,
	path: string,
): Promise<Artifact> => {
	try {
		return await syscall("include", stackFrame, path);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

declare global {
	/** Decode a json string to a value. */
	function syscall(syscall: "json_decode", value: string): unknown;

	/** Encode a value to a json string. */
	function syscall(syscall: "json_encode", value: any): string;
}

export let json = {
	decode: (value: string): unknown => {
		try {
			return syscall("json_decode", value);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	encode: (value: any): string => {
		try {
			return syscall("json_encode", value);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	function syscall(syscall: "log", value: string): void;
}

export let log = (value: string) => {
	try {
		return syscall("log", value);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

declare global {
	function syscall(
		syscall: "operation_get",
		hash: OperationHash,
	): Promise<Operation>;

	function syscall(
		syscall: "operation_run",
		operation: Operation,
	): Promise<Value>;
}

export let operation = {
	get: async (hash: OperationHash): Promise<Operation> => {
		try {
			return await syscall("operation_get", hash);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	run: async (operation: Operation): Promise<Value> => {
		try {
			return await syscall("operation_run", operation);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	function syscall(
		syscall: "process_new",
		system: System,
		executable: Template,
		env?: Record<string, Template> | nullish,
		args?: Array<Template> | nullish,
		checksum?: Checksum | nullish,
		unsafe?: boolean | nullish,
		network?: boolean | nullish,
		hostPaths?: Array<string> | nullish,
	): Promise<Process>;
}

export let process = {
	new: async (
		system: System,
		executable: Template,
		env: Record<string, Template>,
		args: Array<Template>,
		checksum: Checksum | nullish,
		unsafe: boolean,
		network: boolean,
		hostPaths: Array<string>,
	): Promise<Process> => {
		try {
			return await syscall(
				"process_new",
				system,
				executable,
				env,
				args,
				checksum,
				unsafe,
				network,
				hostPaths,
			);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	function syscall(syscall: "stack_frame", index: number): StackFrame;
}

export let stackFrame = (index: number): StackFrame => {
	try {
		return syscall("stack_frame", index + 1);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

declare global {
	function syscall(syscall: "symlink_new", target: Template): Promise<Symlink>;
}

export let symlink = {
	new: async (target: Template): Promise<Symlink> => {
		try {
			return await syscall("symlink_new", target);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	/** Decode a toml string to a value. */
	function syscall(syscall: "toml_decode", value: string): unknown;

	/** Encode a value to a toml string. */
	function syscall(syscall: "toml_encode", value: any): string;
}

export let toml = {
	decode: (value: string): unknown => {
		try {
			return syscall("toml_decode", value);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	encode: (value: any): string => {
		try {
			return syscall("toml_encode", value);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	/** Decode UTF-8 bytes to a string. */
	function syscall(syscall: "utf8_decode", value: Uint8Array): string;

	/** Encode a string to UTF-8 bytes. */
	function syscall(syscall: "utf8_encode", value: string): Uint8Array;
}

export let utf8 = {
	decode: (value: Uint8Array): string => {
		try {
			return syscall("utf8_decode", value);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	encode: (value: string): Uint8Array => {
		try {
			return syscall("utf8_encode", value);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	/** Decode a yaml string to a value. */
	function syscall(syscall: "yaml_decode", value: string): unknown;

	/** Encode a value to a yaml string. */
	function syscall(syscall: "yaml_encode", value: any): string;
}

export let yaml = {
	decode: (value: string): unknown => {
		try {
			return syscall("yaml_decode", value);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	encode: (value: any): string => {
		try {
			return syscall("yaml_encode", value);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};
