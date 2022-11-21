interface ImportMeta {
	url: string;
}

declare namespace Tangram {
	export enum Syscall {
		Print = "print",
		Serialize = "serialize",
		Deserialize = "deserialize",
		AddBlob = "add_blob",
		GetBlob = "get_blob",
		AddExpression = "add_expression",
		GetExpression = "get_expression",
		Evaluate = "evaluate",
	}

	function syscall(syscall: Syscall.Print, value: string): void;
	function syscall<T>(
		syscall: Syscall.Serialize,
		format: string,
		value: T,
	): Uint8Array;
	function syscall<T>(
		syscall: Syscall.Deserialize,
		format: string,
		value: string | Uint8Array,
	): T;
	function syscall(syscall: Syscall.AddBlob, blob: Uint8Array): Promise<Hash>;
	function syscall(syscall: Syscall.GetBlob, hash: Hash): Promise<Uint8Array>;
	function syscall(
		syscall: Syscall.AddExpression,
		expression: Expression,
	): Promise<Hash>;
	function syscall(
		syscall: Syscall.GetExpression,
		hash: Hash,
	): Promise<Expression>;
	function syscall(syscall: Syscall.Evaluate, hash: Hash): Promise<Hash>;

	export type Hash = string;

	export enum ExpressionType {
		Null = "null",
		Bool = "bool",
		Number = "number",
		String = "string",
		Directory = "directory",
		File = "file",
		Symlink = "symlink",
		Dependency = "dependency",
		Template = "template",
		Package = "package",
		Js = "js",
		Fetch = "fetch",
		Process = "process",
		Target = "target",
		Array = "array",
		Map = "map",
	}

	export type Expression =
		| {
				type: ExpressionType.Null;
				value: null;
		  }
		| {
				type: ExpressionType.Bool;
				value: boolean;
		  }
		| {
				type: ExpressionType.Number;
				value: number;
		  }
		| {
				type: ExpressionType.String;
				value: string;
		  }
		| {
				type: ExpressionType.Directory;
				value: Directory;
		  }
		| {
				type: ExpressionType.File;
				value: File;
		  }
		| {
				type: ExpressionType.Symlink;
				value: Symlink;
		  }
		| {
				type: ExpressionType.Dependency;
				value: Dependency;
		  }
		| {
				type: ExpressionType.Package;
				value: Package;
		  }
		| {
				type: ExpressionType.Template;
				value: Template;
		  }
		| {
				type: ExpressionType.Fetch;
				value: Fetch;
		  }
		| {
				type: ExpressionType.Js;
				value: Js;
		  }
		| {
				type: ExpressionType.Process;
				value: Process;
		  }
		| {
				type: ExpressionType.Target;
				value: Target;
		  }
		| {
				type: ExpressionType.Array;
				value: _Array;
		  }
		| {
				type: ExpressionType.Map;
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

	export enum System {
		Amd64Linux = "amd64_linux",
		Amd64Macos = "amd64_macos",
		Arm64Linux = "arm64_linux",
		Arm64Macos = "arm64_macos",
	}

	export type Digest = {
		algorithm: DigestAlgorithm;
		encoding: DigestEncoding;
		value: string;
	};

	export enum DigestAlgorithm {
		Sha256 = "sha256",
	}

	export enum DigestEncoding {
		Hexadecimal = "hexadecimal",
	}
}
