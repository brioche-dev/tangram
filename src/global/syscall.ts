declare global {
	function syscall(
		syscall: "artifact_bundle",
		artifact: Artifact,
	): Promise<Artifact>;

	function syscall(syscall: "artifact_get", block: Block): Promise<Artifact>;

	function syscall(syscall: "blob_bytes", blob: Blob): Promise<Uint8Array>;

	function syscall(syscall: "blob_get", block: Block): Promise<Blob>;

	function syscall(syscall: "blob_new", arg: Blob.Arg): Promise<Blob>;

	function syscall(syscall: "blob_text", blob: Blob): Promise<string>;

	function syscall(syscall: "block_bytes", block: Block): Promise<Uint8Array>;

	function syscall(
		syscall: "block_children",
		block: Block,
	): Promise<Array<Block>>;

	function syscall(syscall: "block_data", block: Block): Promise<Uint8Array>;

	function syscall(syscall: "block_new", arg: Block.Arg): Block;

	function syscall(
		syscall: "checksum",
		algorithm: ChecksumAlgorithm,
		bytes: string | Uint8Array,
	): Checksum;

	function syscall(syscall: "directory_new", arg: Directory.Arg): Directory;

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

	function syscall(syscall: "file_new", arg: File.Arg): File;

	function syscall(syscall: "log", value: string): void;

	function syscall(syscall: "operation_get", block: Block): Promise<Operation>;

	function syscall(
		syscall: "operation_evaluate",
		operation: Operation,
	): Promise<Value>;

	function syscall(syscall: "resource_new", arg: Resource.Arg): Resource;

	function syscall(syscall: "symlink_new", arg: Symlink.Arg): Symlink;

	function syscall(syscall: "target_new", arg: Target.Arg): Target;

	function syscall(syscall: "task_new", arg: Task.Arg): Task;
}

export type Artifact =
	| { kind: "directory"; value: Directory }
	| { kind: "file"; value: File }
	| { kind: "symlink"; value: Symlink };

export type Blob = {
	block: Block;
	kind: Blob.Kind;
};

export namespace Blob {
	export type Arg = {
		children: Array<Block>;
	};

	export type Kind =
		| { kind: "branch"; value: Array<[Block, number]> }
		| { kind: "leaf"; value: number };
}

export type Block = {
	id: Id;
};

export namespace Block {
	export type Arg = {
		children: Array<Block>;
		data: Uint8Array | string;
	};
}

export type Checksum = string;

export type ChecksumAlgorithm = "blake3" | "sha256" | "sha512";

export type Directory = {
	block: Block;
	entries: Record<string, Block>;
};

export namespace Directory {
	export type Arg = {
		entries: Record<string, Artifact>;
	};
}

export type File = {
	block: Block;
	contents: Block;
	executable: boolean;
	references: Array<Block>;
};

export namespace File {
	export type Arg = {
		contents: Blob;
		executable: boolean;
		references: Array<Artifact>;
	};
}

export type Id = string;

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
	package: Block;
	modulePath: Subpath;
};

export type Package = {
	artifact: Artifact;
};

export type Position = {
	line: number;
	character: number;
};

export type Operation =
	| { kind: "resource"; value: Resource }
	| { kind: "target"; value: Target }
	| { kind: "task"; value: Task };

export type Relpath = string;

export type Subpath = string;

export type Placeholder = {
	name: string;
};

export type Resource = {
	block: Block;
	url: string;
	unpack?: UnpackFormat;
	checksum?: Checksum;
	unsafe: boolean;
};

export namespace Resource {
	export type Arg = {
		url: string;
		unpack?: UnpackFormat;
		checksum?: Checksum;
		unsafe: boolean;
	};
}

export type UnpackFormat =
	| ".tar"
	| ".tar.bz2"
	| ".tar.gz"
	| ".tar.lz"
	| ".tar.xz"
	| ".tar.zstd"
	| ".zip";

export type Symlink = {
	block: Block;
	target: Template;
};

export namespace Symlink {
	export type Arg = { target: Template };
}

export type Target = {
	block: Block;
	package: Block;
	modulePath: Subpath;
	name: string;
	env?: Record<string, Value>;
	args?: Array<Value>;
};

export namespace Target {
	export type Arg = {
		package: Block;
		modulePath: Subpath;
		name: string;
		env: Record<string, Value>;
		args: Array<Value>;
	};
}

export type Task = {
	block: Block;
	system: System;
	executable: Template;
	env: Record<string, Template>;
	args: Array<Template>;
	checksum: Checksum | undefined;
	unsafe: boolean;
	network: boolean;
};

export namespace Task {
	export type Arg = {
		system: System;
		executable: Template;
		env?: Record<string, Template>;
		args?: Array<Template>;
		checksum?: Checksum;
		unsafe?: boolean;
		network?: boolean;
	};
}

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
	| { kind: "block"; value: Block }
	| { kind: "blob"; value: Blob }
	| { kind: "artifact"; value: Artifact }
	| { kind: "placeholder"; value: Placeholder }
	| { kind: "template"; value: Template }
	| { kind: "operation"; value: Operation }
	| { kind: "array"; value: Array<Value> }
	| { kind: "object"; value: Record<string, Value> };

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

	new: async (arg: Blob.Arg): Promise<Blob> => {
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

	new: async (arg: Block.Arg): Promise<Block> => {
		try {
			return syscall("block_new", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

export let checksum = (
	algorithm: ChecksumAlgorithm,
	bytes: string | Uint8Array,
): Checksum => {
	try {
		return syscall("checksum", algorithm, bytes);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let directory = {
	new: (arg: Directory.Arg): Directory => {
		try {
			return syscall("directory_new", arg);
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
	new: (arg: File.Arg): File => {
		try {
			return syscall("file_new", arg);
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

	evaluation: async (operation: Operation): Promise<Value> => {
		try {
			return await syscall("operation_evaluate", operation);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

export let resource = {
	new: (arg: Resource.Arg): Resource => {
		try {
			return syscall("resource_new", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

export let target = {
	new: (arg: Target.Arg): Target => {
		try {
			return syscall("target_new", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

export let task = {
	new: (arg: Task.Arg): Task => {
		try {
			return syscall("task_new", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

export let symlink = {
	new: (arg: Symlink.Arg): Symlink => {
		try {
			return syscall("symlink_new", arg);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};
