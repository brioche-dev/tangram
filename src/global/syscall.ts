import { Artifact } from "./artifact.ts";
import { Blob } from "./blob.ts";
import { Block } from "./block.ts";
import { Checksum } from "./checksum.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Operation } from "./operation.ts";
import { Subpath } from "./path.ts";
import { Resource } from "./resource.ts";
import { Symlink } from "./symlink.ts";
import { System } from "./system.ts";
import { Target } from "./target.ts";
import { Task } from "./task.ts";
import { Template } from "./template.ts";
import { Value } from "./value.ts";

declare global {
	function syscall(
		syscall: "artifact_bundle",
		artifact: Artifact,
	): Promise<Artifact>;

	function syscall(syscall: "artifact_get", block: Block): Promise<Artifact>;

	function syscall(syscall: "blob_bytes", blob: Blob): Promise<Uint8Array>;

	function syscall(syscall: "blob_get", block: Block): Promise<Blob>;

	function syscall(syscall: "blob_new", arg: BlobArg): Promise<Blob>;

	function syscall(syscall: "blob_text", blob: Blob): Promise<string>;

	function syscall(syscall: "block_bytes", block: Block): Promise<Uint8Array>;

	function syscall(
		syscall: "block_children",
		block: Block,
	): Promise<Array<Block>>;

	function syscall(syscall: "block_data", block: Block): Promise<Uint8Array>;

	function syscall(syscall: "block_new", arg: BlockArg): Promise<Block>;

	function syscall(
		syscall: "checksum",
		algorithm: Checksum.Algorithm,
		bytes: string | Uint8Array,
	): Checksum;

	function syscall(
		syscall: "directory_new",
		arg: DirectoryArg,
	): Promise<Directory>;

	function syscall(
		syscall: "encoding_base64_decode",
		value: string,
	): Uint8Array;

	function syscall(
		syscall: "encoding_base64_encode",
		value: Uint8Array,
	): string;

	function syscall(syscall: "encoding_hex_decode", value: string): Uint8Array;

	function syscall(syscall: "encoding_hex_encode", value: Uint8Array): string;

	function syscall(syscall: "encoding_json_decode", value: string): unknown;

	function syscall(syscall: "encoding_json_encode", value: any): string;

	function syscall(syscall: "encoding_toml_decode", value: string): unknown;

	function syscall(syscall: "encoding_toml_encode", value: any): string;

	function syscall(syscall: "encoding_utf8_decode", value: Uint8Array): string;

	function syscall(syscall: "encoding_utf8_encode", value: string): Uint8Array;

	function syscall(syscall: "encoding_yaml_decode", value: string): unknown;

	function syscall(syscall: "encoding_yaml_encode", value: any): string;

	function syscall(syscall: "file_new", arg: FileArg): Promise<File>;

	function syscall(syscall: "log", value: string): void;

	function syscall(syscall: "operation_get", block: Block): Promise<Operation>;

	function syscall(
		syscall: "operation_evaluate",
		operation: Operation,
	): Promise<Value>;

	function syscall(
		syscall: "resource_new",
		arg: ResourceArg,
	): Promise<Resource>;

	function syscall(syscall: "symlink_new", arg: SymlinkArg): Promise<Symlink>;

	function syscall(syscall: "target_new", arg: TargetArg): Promise<Target>;

	function syscall(syscall: "task_new", arg: TaskArg): Promise<Task>;
}

export type BlobArg = {
	children: Array<Block>;
};

export type BlockArg = {
	children: Array<Block> | undefined;
	data: Uint8Array | undefined;
};

export type DirectoryArg = {
	entries: Record<string, Artifact>;
};

export type FileArg = {
	contents: Blob;
	executable: boolean;
	references: Array<Artifact>;
};

export type ResourceArg = {
	url: string;
	unpack?: Resource.UnpackFormat;
	checksum?: Checksum;
	unsafe: boolean;
};

export type SymlinkArg = { target: Template };

export type TargetArg = {
	package: Block;
	path: Subpath;
	name: string;
	env: Record<string, Value>;
	args: Array<Value>;
};

export type TaskArg = {
	host: System;
	executable: Template;
	env?: Record<string, Template>;
	args?: Array<Template>;
	checksum?: Checksum;
	unsafe?: boolean;
	network?: boolean;
};

export let artifact = {
	bundle: async (artifact: Artifact): Promise<Artifact> => {
		try {
			return await syscall("artifact_bundle", artifact);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	get: async (block: Block): Promise<Artifact> => {
		try {
			return await syscall("artifact_get", block);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

export let blob = {
	bytes: async (blob: Blob): Promise<Uint8Array> => {
		try {
			return await syscall("blob_bytes", blob);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	get: async (arg: Block): Promise<Blob> => {
		try {
			return await syscall("blob_get", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	new: async (arg: BlobArg): Promise<Blob> => {
		try {
			return await syscall("blob_new", arg);
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

export let block = {
	bytes: async (block: Block): Promise<Uint8Array> => {
		try {
			return await syscall("block_bytes", block);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	children: async (block: Block): Promise<Array<Block>> => {
		try {
			return syscall("block_children", block);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	data: async (block: Block): Promise<Uint8Array> => {
		try {
			return await syscall("block_data", block);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	new: async (arg: BlockArg): Promise<Block> => {
		try {
			return syscall("block_new", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

export let checksum = (
	algorithm: Checksum.Algorithm,
	bytes: string | Uint8Array,
): Checksum => {
	try {
		return syscall("checksum", algorithm, bytes);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let directory = {
	new: async (arg: DirectoryArg): Promise<Directory> => {
		try {
			return await syscall("directory_new", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

export namespace encoding {
	export let base64 = {
		decode: (value: string): Uint8Array => {
			try {
				return syscall("encoding_base64_decode", value);
			} catch (cause) {
				throw new Error("The syscall failed.", { cause });
			}
		},

		encode: (value: Uint8Array): string => {
			try {
				return syscall("encoding_base64_encode", value);
			} catch (cause) {
				throw new Error("The syscall failed.", { cause });
			}
		},
	};

	export let hex = {
		decode: (value: string): Uint8Array => {
			try {
				return syscall("encoding_hex_decode", value);
			} catch (cause) {
				throw new Error("The syscall failed.", { cause });
			}
		},

		encode: (value: Uint8Array): string => {
			try {
				return syscall("encoding_hex_encode", value);
			} catch (cause) {
				throw new Error("The syscall failed.", { cause });
			}
		},
	};

	export let json = {
		decode: (value: string): unknown => {
			try {
				return syscall("encoding_json_decode", value);
			} catch (cause) {
				throw new Error("The syscall failed.", { cause });
			}
		},

		encode: (value: any): string => {
			try {
				return syscall("encoding_json_encode", value);
			} catch (cause) {
				throw new Error("The syscall failed.", { cause });
			}
		},
	};

	export let toml = {
		decode: (value: string): unknown => {
			try {
				return syscall("encoding_toml_decode", value);
			} catch (cause) {
				throw new Error("The syscall failed.", { cause });
			}
		},

		encode: (value: any): string => {
			try {
				return syscall("encoding_toml_encode", value);
			} catch (cause) {
				throw new Error("The syscall failed.", { cause });
			}
		},
	};

	export let utf8 = {
		decode: (value: Uint8Array): string => {
			try {
				return syscall("encoding_utf8_decode", value);
			} catch (cause) {
				throw new Error("The syscall failed.", { cause });
			}
		},

		encode: (value: string): Uint8Array => {
			try {
				return syscall("encoding_utf8_encode", value);
			} catch (cause) {
				throw new Error("The syscall failed.", { cause });
			}
		},
	};

	export let yaml = {
		decode: (value: string): unknown => {
			try {
				return syscall("encoding_yaml_decode", value);
			} catch (cause) {
				throw new Error("The syscall failed.", { cause });
			}
		},

		encode: (value: any): string => {
			try {
				return syscall("encoding_yaml_encode", value);
			} catch (cause) {
				throw new Error("The syscall failed.", { cause });
			}
		},
	};
}

export let file = {
	new: async (arg: FileArg): Promise<File> => {
		try {
			return await syscall("file_new", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

export let log = (value: string) => {
	try {
		return syscall("log", value);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let operation = {
	get: async (block: Block): Promise<Operation> => {
		try {
			return await syscall("operation_get", block);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	evaluate: async (operation: Operation): Promise<Value> => {
		try {
			return await syscall("operation_evaluate", operation);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

export let resource = {
	new: async (arg: ResourceArg): Promise<Resource> => {
		try {
			return await syscall("resource_new", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

export let target = {
	new: async (arg: TargetArg): Promise<Target> => {
		try {
			return await syscall("target_new", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

export let task = {
	new: async (arg: TaskArg): Promise<Task> => {
		try {
			return await syscall("task_new", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

export let symlink = {
	new: async (arg: SymlinkArg): Promise<Symlink> => {
		try {
			return await syscall("symlink_new", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};
