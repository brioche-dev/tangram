import { Artifact } from "./artifact.ts";
import { Blob } from "./blob.ts";
import { Build } from "./build.ts";
import { Checksum } from "./checksum.ts";
import { Value } from "./value.ts";

declare global {
	function syscall(
		syscall: "artifact_bundle",
		artifact: Artifact,
	): Promise<Artifact>;

	function syscall(syscall: "blob_bytes", blob: Blob): Promise<Uint8Array>;

	function syscall(syscall: "build_output", build: Build): Promise<Value>;

	function syscall(
		syscall: "checksum",
		algorithm: Checksum.Algorithm,
		bytes: string | Uint8Array,
	): Checksum;

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

	function syscall(syscall: "log", value: string): void;

	function syscall(syscall: "value_load", value: Value): Promise<Value>;

	function syscall(syscall: "value_store", value: Value): Promise<Value>;
}

export let artifact = {
	bundle: async (artifact: Artifact): Promise<Artifact> => {
		try {
			return await syscall("artifact_bundle", artifact);
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
};

export let build = {
	output: async (build: Build): Promise<Value> => {
		try {
			return await syscall("build_output", build);
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

export let encoding = {
	base64: {
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
	},

	hex: {
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
	},

	json: {
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
	},

	toml: {
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
	},

	utf8: {
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
	},

	yaml: {
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
	},
};

export let log = (value: string) => {
	try {
		return syscall("log", value);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let value = {
	load: async (value: Value): Promise<Value> => {
		try {
			return await syscall("value_load", value);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	store: async (value: Value): Promise<Value> => {
		try {
			return await syscall("value_store", value);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};
