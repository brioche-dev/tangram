declare module Tangram {
	module internal {
		enum System {
			Amd64Linux = "amd64_linux",
			Amd64Macos = "amd64_macos",
			Arm64Linux = "arm64_linux",
			Arm64Macos = "arm64_macos",
		}

		type Hash = string;

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
			args: Hash;
			artifact: Hash;
			dependencies: { [key: string]: Hash };
			name: string;
			path: string;
		};

		type Fetch = {
			url: string;
			hash: string | null;
			unpack: boolean;
		};

		type Process = {
			args: Hash;
			command: Hash;
			env: Hash;
			system: System;
		};

		type Target = {
			args: Hash;
			name: string;
			package: Hash;
		};

		type _Array = Array<Hash>;

		type _Map = { [key: string]: Hash };

		enum Syscall {
			Print = "print",
			AddBlob = "add_blob",
			GetBlob = "get_blob",
			AddExpression = "add_expression",
			GetExpression = "get_expression",
			Evaluate = "evaluate",
		}

		function syscall(syscall: Syscall.Print, value: string): void;

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
	}

	enum System {
		Amd64Linux = "amd64_linux",
		Amd64Macos = "amd64_macos",
		Arm64Linux = "arm64_linux",
		Arm64Macos = "arm64_macos",
	}

	class Hash<T extends Expression = Expression> {
		constructor(string: string);

		toString(): string;
	}

	type Expression<Output extends Expression = Expression<any>> =
		| null
		| boolean
		| number
		| string
		| Artifact
		| Directory
		| File
		| Symlink
		| Dependency
		| Package
		| Template
		| Js<Output>
		| Fetch
		| Process
		| Target<Output>
		| Array<Expression<Output>>
		| { [key: string]: Expression<Output> };

	type OutputForExpression<T extends Expression> = [T] extends [null]
		? null
		: [T] extends [boolean]
		? boolean
		: [T] extends [number]
		? number
		: [T] extends [string]
		? string
		: [T] extends [Artifact]
		? Artifact
		: [T] extends [Directory]
		? Directory
		: [T] extends [File]
		? File
		: [T] extends [Symlink]
		? Symlink
		: [T] extends [Dependency]
		? Dependency
		: [T] extends [Package]
		? Package
		: [T] extends [Template]
		? Template
		: [T] extends [Js<infer O>]
		? OutputForExpression<O>
		: [T] extends [Fetch]
		? Artifact
		: [T] extends [Process]
		? Artifact
		: [T] extends [Target<infer O>]
		? OutputForExpression<O>
		: [T] extends [Array<infer V extends Expression>]
		? Array<OutputForExpression<V>>
		: [T] extends [{ [key: string]: Expression<infer V extends Expression> }]
		? { [K in keyof T]: OutputForExpression<T[K]> }
		: [T] extends [Expression<infer O extends Expression>]
		? OutputForExpression<O>
		: never;

	class Artifact {
		#type: "artifact";

		constructor(
			expression: Expression<Directory | File | Symlink | Dependency>,
		);

		getRoot(): Promise<Expression<Directory | File | Symlink | Dependency>>;
	}

	type FilesystemExpression = Directory | File | Symlink | Dependency;

	type DirectoryEntries = {
		[filename: string]: FilesystemExpression;
	};

	class Directory {
		#type: "directory";

		constructor(entries: DirectoryEntries);

		getEntries(): Promise<DirectoryEntries>;
	}

	class File {
		#type: "file";

		constructor(blob: Hash, executable?: boolean);

		executable: boolean;

		getBlob(): Promise<Uint8Array>;
	}

	class Symlink {
		#type: "symlink";

		constructor(target: string);

		target: string;
	}

	class Dependency {
		#type: "dependency";

		constructor(artifact: Artifact, path?: string | null);

		getArtifact(): Promise<Artifact>;
	}

	type PackageArgs = {
		source: Expression<Artifact>;
		dependencies: { [name: string]: Expression<Artifact> };
	};

	class Package {
		#type: "package";

		constructor(args: PackageArgs);

		getSource(): Promise<Artifact>;

		getDependencies(): Promise<{ [name: string]: Package }>;
	}

	class Template {
		#type: "template";

		constructor(components: Array<string | Artifact | Template>);

		getComponents(): Promise<Array<string | Artifact | Template>>;
	}

	type JsArgs = {
		args: Expression<Array<Expression<string | Artifact | Template>>>;
		artifact: Expression<Artifact>;
		path: string;
		dependencies: { [name: string]: Expression<Artifact> };
		export: string;
	};

	class Js<O extends Expression> {
		#type: "js";

		constructor(args: JsArgs);

		getArgs(): Promise<Expression<Array<Expression>>>;
	}

	type FetchArgs = {
		hash?: string;
		unpack?: boolean;
		url: string;
	};

	class Fetch {
		#type: `fetch`;
		url: string;
		hash: string | null;
		unpack: boolean;

		constructor(args: FetchArgs);
	}

	type ProcessArgs = {
		args: Expression<Array<Expression>>;
		command: Expression<string | Artifact | Template>;
		env: Expression<{
			[key: string]: Expression<string | Artifact | Template>;
		}>;
		system: System;
	};

	class Process {
		#type: `process`;
		constructor(args: ProcessArgs);
	}

	type TargetArgs = {
		package: Expression<Artifact>;
		name: string;
		args: Expression<Array<Expression>>;
	};

	class Target<O extends Expression> {
		#type: `target`;
		constructor(args: TargetArgs);
		getPackage(): Promise<Package>;
		getArgs(): Promise<Array<Expression>>;
	}

	let template: (
		strings: TemplateStringsArray,
		...placeholders: Array<Expression<string | Artifact | Template>>
	) => Template;

	let evaluate: <E extends Expression>(
		expression: E,
	) => Promise<OutputForExpression<E>>;

	let addExpression: <E extends Expression>(expression: E) => Promise<Hash<E>>;

	let getExpression: <E extends Expression>(hash: Hash<E>) => Promise<E>;

	let addBlob: (blob: Uint8Array) => Promise<Hash>;

	let getBlob: (hash: Hash) => Promise<Uint8Array>;

	let source: (url: string | URL) => Promise<Package>;
}
