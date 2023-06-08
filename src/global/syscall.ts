export type Artifact =
	| { kind: "directory"; value: Directory }
	| { kind: "file"; value: File }
	| { kind: "symlink"; value: Symlink };

export namespace Artifact {
	export type Hash = string;
}

export type Blob = {
	hash: Blob.Hash;
};

export namespace Blob {
	export type Hash = string;
}

export type Checksum = string;

export type ChecksumAlgorithm = "blake3" | "sha256" | "sha512";

export type Command = {
	hash: Operation.Hash;
	system: System;
	executable: Template;
	env: Record<string, Template>;
	args: Array<Template>;
	checksum: Checksum | undefined;
	unsafe: boolean;
	network: boolean;
	hostPaths: Array<string>;
};

export type Directory = {
	hash: Artifact.Hash;
	entries: Record<string, Artifact.Hash>;
};

export type File = {
	hash: Artifact.Hash;
	blob: Blob;
	executable: boolean;
	references: Array<Artifact.Hash>;
};

export type Function = {
	hash: Operation.Hash;
	packageHash: Package.Hash;
	modulePath: Subpath;
	kind: Function.Kind;
	name: string;
	env?: Record<string, Value>;
	args?: Array<Value>;
};

export namespace Function {
	export type Kind = "function" | "test";
}

export type Module =
	| { kind: "library"; value: LibraryModule }
	| { kind: "document"; value: DocumentModule }
	| { kind: "normal"; value: NormalModule };

export type LibraryModule = {
	modulePath: Subpath;
};

export type DocumentModule = {
	packagePath: string;
	modulePath: Subpath;
};

export type NormalModule = {
	packageHash: Package.Hash;
	modulePath: Subpath;
};

export type Package = {
	artifact: Artifact;
};

export namespace Package {
	export type Hash = string;
}

export type Position = {
	line: number;
	character: number;
};

export type Operation =
	| { kind: "command"; value: Command }
	| { kind: "function"; value: Function }
	| { kind: "resource"; value: Resource };

export namespace Operation {
	export type Hash = string;
}

export type Relpath = string;

export type Subpath = string;

export type Placeholder = {
	name: string;
};

export type Resource = {
	hash: Operation.Hash;
	url: string;
	unpack: boolean;
	checksum?: Checksum;
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

export type Symlink = {
	hash: Artifact.Hash;
	target: Template;
};

export type Template = {
	components: Array<Template.Component>;
};

export namespace Template {
	export type Component =
		| { kind: "string"; value: string }
		| { kind: "artifact"; value: Artifact }
		| { kind: "placeholder"; value: Placeholder };
}

export type System =
	| "amd64_linux"
	| "arm64_linux"
	| "amd64_macos"
	| "arm64_macos";

export type Value =
	| { kind: "null" }
	| { kind: "bool"; value: boolean }
	| { kind: "number"; value: number }
	| { kind: "string"; value: string }
	| { kind: "bytes"; value: Uint8Array }
	| { kind: "subpath"; value: Subpath }
	| { kind: "relpath"; value: Relpath }
	| { kind: "blob"; value: Blob }
	| { kind: "artifact"; value: Artifact }
	| { kind: "placeholder"; value: Placeholder }
	| { kind: "template"; value: Template }
	| { kind: "operation"; value: Operation }
	| { kind: "array"; value: Array<Value> }
	| { kind: "object"; value: Record<string, Value> };

declare global {
	function syscall(
		syscall: "artifact_bundle",
		artifact: Artifact,
	): Promise<Artifact>;

	function syscall(
		syscall: "artifact_get",
		hash: Artifact.Hash,
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

	get: async (hash: Artifact.Hash): Promise<Artifact> => {
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
	type CommandArg = {
		system: System;
		executable: Template;
		env?: Record<string, Template>;
		args?: Array<Template>;
		checksum?: Checksum;
		unsafe?: boolean;
		network?: boolean;
		hostPaths?: Array<string>;
	};

	function syscall(syscall: "command_new", arg: CommandArg): Command;
}

export let command = {
	new: (arg: CommandArg): Command => {
		try {
			return syscall("command_new", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	type DirectoryArg = {
		entries: Record<string, Artifact>;
	};

	function syscall(syscall: "directory_new", arg: DirectoryArg): Directory;
}

export let directory = {
	new: (arg: DirectoryArg): Directory => {
		try {
			return syscall("directory_new", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	type FileArg = {
		blob: Blob;
		executable: boolean;
		references: Array<Artifact>;
	};

	function syscall(syscall: "file_new", arg: FileArg): File;
}

export let file = {
	new: (arg: FileArg): File => {
		try {
			return syscall("file_new", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	type FunctionArg = {
		packageHash: Package.Hash;
		modulePath: Subpath;
		kind: Function.Kind;
		name: string;
		env: Record<string, Value>;
		args: Array<Value>;
	};

	function syscall(syscall: "function_new", arg: FunctionArg): Function;
}

let function_ = {
	new: (arg: FunctionArg): Function => {
		try {
			return syscall("function_new", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};
export { function_ as function };

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
		hash: Operation.Hash,
	): Promise<Operation>;

	function syscall(
		syscall: "operation_run",
		operation: Operation,
	): Promise<Value>;
}

export let operation = {
	get: async (hash: Operation.Hash): Promise<Operation> => {
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
	type ResourceArg = {
		url: string;
		unpack: boolean;
		checksum?: Checksum;
		unsafe: boolean;
	};

	function syscall(syscall: "resource_new", arg: ResourceArg): Resource;
}

export let resource = {
	new: (arg: ResourceArg): Resource => {
		try {
			return syscall("resource_new", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	type SymlinkArg = { target: Template };

	function syscall(syscall: "symlink_new", arg: SymlinkArg): Symlink;
}

export let symlink = {
	new: (arg: SymlinkArg): Symlink => {
		try {
			return syscall("symlink_new", arg);
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
