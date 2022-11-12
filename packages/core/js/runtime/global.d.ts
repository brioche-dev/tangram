interface ImportMeta {
	url: string;
}

declare namespace Tangram {
	enum Syscall {
		Print = "print",
		Deserialize = "deserialize",
		AddBlob = "add_blob",
		GetBlob = "get_blob",
		AddExpression = "add_expression",
		GetExpression = "get_expression",
		Evaluate = "evaluate",
	}

	function syscall(syscall: Syscall.Print, value: string): void;

	function syscall(
		syscall: Syscall.Deserialize,
		format: string,
		content: string,
	): any;

	function syscall(syscall: Syscall.AddBlob, blob: Uint8Array): Promise<Hash>;

	function syscall(syscall: Syscall.GetBlob, hash: Hash): Promise<Uint8Array>;

	function syscall(
		syscall: Syscall.AddExpression,
		expression: Expression,
	): Hash;

	function syscall(
		syscall: Syscall.GetExpression,
		hash: Hash,
	): Promise<Expression>;

	function syscall(syscall: Syscall.Evaluate, hash: Hash): Promise<Hash>;

	enum System {
		Amd64Linux = "amd64_linux",
		Amd64Macos = "amd64_macos",
		Arm64Linux = "arm64_linux",
		Arm64Macos = "arm64_macos",
	}

	type Hash = string;

	type Digest = {
		algorithm: DigestAlgorithm;
		encoding: DigestEncoding;
		value: string;
	};

	enum DigestAlgorithm {
		Sha256 = "sha256",
	}

	enum DigestEncoding {
		Hexadecimal = "hexadecimal",
	}

	enum ExpressionType {
		Null = "null",
		Bool = "bool",
		Number = "number",
		String = "string",
		Artifact = "artifact",
		Directory = "directory",
		File = "file",
		Symlink = "symlink",
		Dependency = "dependency",
		Package = "package",
		Template = "template",
		Js = "js",
		Fetch = "fetch",
		Process = "process",
		Target = "target",
		Array = "array",
		Map = "map",
	}

	type Expression =
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
				type: ExpressionType.Artifact;
				value: Artifact;
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

	type Artifact = {
		root: Hash;
	};

	type Directory = {
		entries: { [key: string]: Hash };
	};

	type File = {
		blob: Hash;
		executable: boolean;
	};

	type Symlink = {
		target: string;
	};

	type Dependency = {
		artifact: Hash;
		path: string | null;
	};

	type Package = {
		source: Hash;
		dependencies: { [key: string]: Hash };
	};

	type Template = {
		components: Array<Hash>;
	};

	type Js = {
		package: Hash;
		name: string;
		path: string;
		args: Hash;
	};

	type Fetch = {
		url: string;
		digest: Digest | null;
		unpack: boolean;
	};

	type Process = {
		system: System;
		env: Hash;
		command: Hash;
		args: Hash;
		digest: Digest | null;
		unsafe: boolean | null;
		network: boolean | null;
	};

	type Target = {
		args: Hash;
		name: string;
		package: Hash;
	};

	type _Array = Array<Hash>;

	type _Map = { [key: string]: Hash };
}
