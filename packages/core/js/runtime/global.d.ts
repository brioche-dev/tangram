interface ImportMeta {
	url: string;
}

declare namespace Tangram {
	export type Syscall =
		| "print"
		| "serialize"
		| "deserialize"
		| "add_blob"
		| "get_blob"
		| "add_expression"
		| "get_expression"
		| "evaluate";

	function syscall(syscall: "print", value: string): void;
	function syscall<T>(
		syscall: "serialize",
		format: string,
		value: T,
	): Uint8Array;
	function syscall<T>(
		syscall: "deserialize",
		format: string,
		value: string | Uint8Array,
	): T;
	function syscall(syscall: "add_blob", blob: Uint8Array): Promise<Hash>;
	function syscall(syscall: "get_blob", hash: Hash): Promise<Uint8Array>;
	function syscall(
		syscall: "add_expression",
		expression: Expression,
	): Promise<Hash>;
	function syscall(syscall: "get_expression", hash: Hash): Promise<Expression>;
	function syscall(syscall: "evaluate", hash: Hash): Promise<Hash>;

	export type Hash = string;

	export type ExpressionType =
		| "null"
		| "bool"
		| "number"
		| "string"
		| "directory"
		| "file"
		| "symlink"
		| "dependency"
		| "template"
		| "package"
		| "js"
		| "fetch"
		| "process"
		| "target"
		| "array"
		| "map";

	export type Expression =
		| {
				type: "null";
				value: null;
		  }
		| {
				type: "bool";
				value: boolean;
		  }
		| {
				type: "number";
				value: number;
		  }
		| {
				type: "string";
				value: string;
		  }
		| {
				type: "directory";
				value: Directory;
		  }
		| {
				type: "file";
				value: File;
		  }
		| {
				type: "symlink";
				value: Symlink;
		  }
		| {
				type: "dependency";
				value: Dependency;
		  }
		| {
				type: "package";
				value: Package;
		  }
		| {
				type: "template";
				value: Template;
		  }
		| {
				type: "fetch";
				value: Fetch;
		  }
		| {
				type: "js";
				value: Js;
		  }
		| {
				type: "process";
				value: Process;
		  }
		| {
				type: "target";
				value: Target;
		  }
		| {
				type: "array";
				value: _Array;
		  }
		| {
				type: "map";
				value: _Map;
		  };

	export type Directory = {
		entries: { [key: string]: Hash };
	};

	export type File = {
		blob: Hash;
		executable: boolean;
	};

	export type Symlink = {
		target: string;
	};

	export type Dependency = {
		artifact: Hash;
		path: string | null;
	};

	export type Package = {
		source: Hash;
		dependencies: { [key: string]: Hash };
	};

	export type Template = {
		components: Array<Hash>;
	};

	export type Js = {
		package: Hash;
		name: string;
		path: string;
		args: Hash;
	};

	export type Fetch = {
		url: string;
		digest: Digest | null;
		unpack: boolean;
	};

	export type Process = {
		system: System;
		base: Hash | null;
		env: Hash;
		command: Hash;
		args: Hash;
		digest: Digest | null;
		unsafe: boolean | null;
		network: boolean | null;
	};

	export type Target = {
		args: Hash;
		name: string;
		package: Hash;
	};

	export type _Array = Array<Hash>;

	export type _Map = Record<string, Hash>;

	export type System =
		| "amd64_linux"
		| "amd64_macos"
		| "arm64_linux"
		| "arm64_macos";

	export type Digest = {
		algorithm: DigestAlgorithm;
		encoding: DigestEncoding;
		value: string;
	};

	export type DigestAlgorithm = "sha256";

	export type DigestEncoding = "hexadecimal";

	export const typeSymbol: unique symbol;
}
