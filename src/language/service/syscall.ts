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

declare global {
	/** Get the modules for all documents. */
	function syscall(name: "documents"): Array<Module>;
}

export let documents = (): Array<Module> => {
	try {
		return syscall("documents");
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

declare global {
	/** Decode a hex string to bytes. */
	function syscall(syscall: "hex_decode", hex: string): Uint8Array;

	/** Encode bytes to a hex string. */
	function syscall(syscall: "hex_encode", bytes: Uint8Array): string;
}

export let hex = {
	decode: (hex: string): Uint8Array => {
		try {
			return syscall("hex_decode", hex);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	encode: (bytes: Uint8Array): string => {
		try {
			return syscall("hex_encode", bytes);
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
	/** Write to the log. */
	function syscall(name: "log", value: string): void;
}

export let log = (value: string) => {
	try {
		return syscall("log", value);
	} catch (cause) {
		throw new Error("The syscall failed.", { cause });
	}
};

declare global {
	/** Load the text of a module. */
	function syscall(name: "module_load", module: Module): string;

	/** Resolve a specifier from a module. */
	function syscall(
		name: "module_resolve",
		referrer: Module,
		specifier: string,
	): Module;

	/** Get the version of a module. */
	function syscall(name: "module_version", module: Module): string;
}

export let module_ = {
	load: (module: Module): string => {
		try {
			return syscall("module_load", module);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	resolve: (referrer: Module, specifier: string): Module => {
		try {
			return syscall("module_resolve", referrer, specifier);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	version: (module: Module) => {
		try {
			return syscall("module_version", module);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};

declare global {
	/** Decode bytes as UTF-8. */
	function syscall(syscall: "utf8_decode", bytes: Uint8Array): string;

	/** Encode a string as UTF-8. */
	function syscall(syscall: "utf8_encode", string: string): Uint8Array;
}

export let utf8 = {
	decode: (bytes: Uint8Array): string => {
		try {
			return syscall("utf8_decode", bytes);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},

	encode: (string: string): Uint8Array => {
		try {
			return syscall("utf8_encode", string);
		} catch (cause) {
			throw new Error("The syscall failed.", { cause });
		}
	},
};
