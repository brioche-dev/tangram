export type ModuleIdentifier = {
	source: ModuleIdentifierSource;
	path: string;
};

export type ModuleIdentifierSource =
	| { kind: "lib" }
	| { kind: "path"; value: string }
	| { kind: "instance"; value: string };

declare global {
	/** Decode a hex string to bytes. */
	function syscall(syscall: "decode_hex", hex: string): Uint8Array;

	/** Decode bytes as UTF-8. */
	function syscall(syscall: "decode_utf8", bytes: Uint8Array): string;

	/** Encode bytes to a hex string. */
	function syscall(syscall: "encode_hex", bytes: Uint8Array): string;

	/** Encode a string as UTF-8. */
	function syscall(syscall: "encode_utf8", string: string): Uint8Array;

	/** Get the module identifiers of all documents. */
	function syscall(name: "get_documents"): Array<ModuleIdentifier>;

	/** Load the text of a module. */
	function syscall(
		name: "load_module",
		moduleIdentifier: ModuleIdentifier,
	): string;

	/** Write to the log. */
	function syscall(name: "log", value: string): void;

	/** Resolve a module specifier from a module identifier. */
	function syscall(
		name: "resolve_module",
		specifier: string,
		referrer: ModuleIdentifier,
	): ModuleIdentifier;

	/** Get the version of a module. */
	function syscall(
		name: "get_module_version",
		moduleIdentifier: ModuleIdentifier,
	): string;
}

export let decodeHex = (hex: string): Uint8Array => {
	try {
		return syscall("decode_hex", hex);
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

export let encodeHex = (bytes: Uint8Array): string => {
	try {
		return syscall("encode_hex", bytes);
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

export let getDocuments = (): Array<ModuleIdentifier> => {
	try {
		return syscall("get_documents");
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let loadModule = (moduleIdentifier: ModuleIdentifier): string => {
	try {
		return syscall("load_module", moduleIdentifier);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let log = (value: string) => {
	try {
		return syscall("log", value);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let resolveModule = (
	specifier: string,
	referrer: ModuleIdentifier,
): ModuleIdentifier => {
	try {
		return syscall("resolve_module", specifier, referrer);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

export let getModuleVersion = (moduleIdentifier: ModuleIdentifier) => {
	try {
		return syscall("get_module_version", moduleIdentifier);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};
