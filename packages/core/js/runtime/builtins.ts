let textEncoder = new TextEncoder();

export enum System {
	Amd64Linux = "amd64_linux",
	Amd64Macos = "amd64_macos",
	Arm64Linux = "arm64_linux",
	Arm64Macos = "arm64_macos",
}

export enum ExpressionType {
	Null = "null",
	Bool = "bool",
	Number = "number",
	String = "string",
	Artifact = "artifact",
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

type AnyExpression =
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
	| Js<AnyExpression>
	| Fetch
	| Process
	| Target<AnyExpression>
	| Array<AnyExpression>
	| { [key: string]: AnyExpression };

export type Expression<O extends AnyExpression> =
	| ExpressionShallow<O>
	| Target<Expression<O>>
	| Js<Expression<O>>;

type ExpressionShallow<O extends AnyExpression> = O extends null
	? null
	: O extends boolean
	? boolean
	: O extends number
	? number
	: O extends string
	? string
	: O extends Artifact
	? Artifact | Fetch | Process
	: O extends Directory
	? Directory
	: O extends File
	? File
	: O extends Symlink
	? Symlink
	: O extends Dependency
	? Dependency
	: O extends Package
	? Package
	: O extends Template
	? Template
	: O extends Fetch
	? Fetch
	: O extends Process
	? Process
	: O extends Array<infer T extends AnyExpression>
	? Array<Expression<T>>
	: O extends { [key: string]: AnyExpression }
	? { [K in keyof O]: Expression<O[K]> }
	: never;

export type OutputForExpression<E extends AnyExpression> = E extends null
	? null
	: E extends boolean
	? boolean
	: E extends number
	? number
	: E extends string
	? string
	: E extends Artifact
	? Artifact
	: E extends Directory
	? Directory
	: E extends File
	? File
	: E extends Symlink
	? Symlink
	: E extends Dependency
	? Dependency
	: E extends Package
	? Package
	: E extends Template
	? Template
	: E extends Js<infer O extends AnyExpression>
	? OutputForExpression<O>
	: E extends Fetch
	? Artifact
	: E extends Process
	? Artifact
	: E extends Target<infer O extends AnyExpression>
	? OutputForExpression<O>
	: E extends Array<infer T extends AnyExpression>
	? Array<OutputForExpression<T>>
	: E extends { [key: string]: AnyExpression }
	? { [K in keyof E]: OutputForExpression<E[K]> }
	: never;

export type HashOrExpression<E extends AnyExpression> =
	| Hash<Expression<E>>
	| Expression<E>;

export type Digest = Tangram.Digest;

export enum DigestAlgorithm {
	Sha256 = "sha256",
}

export enum DigestEncoding {
	Hexadecimal = "hexadecimal",
}

export class Hash<T extends AnyExpression = AnyExpression> {
	#string: string;

	constructor(string: string) {
		this.#string = string;
	}

	toString() {
		return this.#string;
	}
}

export class Artifact {
	#tangram = "artifact";
	root: HashOrExpression<Directory | File>;

	constructor(root: HashOrExpression<Directory | File>) {
		this.root = root;
	}

	static fromJson(artifact: Tangram.Artifact): Artifact {
		let root = new Hash(artifact.root);
		return new Artifact(root);
	}

	async toJson(): Promise<Tangram.Expression> {
		let root = await addExpression(this.root);
		return {
			type: ExpressionType.Artifact,
			value: {
				root: root.toString(),
			},
		};
	}

	async getRoot(): Promise<Expression<Directory | File>> {
		return await getExpression(this.root);
	}
}

type DirectoryEntries = {
	[filename: string]: HashOrExpression<Directory | File | Symlink | Dependency>;
};

export class Directory {
	#tangram = "directory";
	#entries: DirectoryEntries;

	constructor(entries: DirectoryEntries) {
		this.#entries = entries;
	}

	static fromJson(directory: Tangram.Directory): Directory {
		let entries = Object.fromEntries(
			Object.entries(directory.entries).map(([key, value]) => [
				key,
				new Hash(value),
			]),
		);
		return new Directory(entries);
	}

	async toJson(): Promise<Tangram.Expression> {
		let entries = Object.fromEntries(
			await Promise.all(
				Object.entries(this.#entries).map(async ([key, value]) => [
					key,
					(await addExpression(value)).toString(),
				]),
			),
		);
		return {
			type: ExpressionType.Directory,
			value: { entries },
		};
	}

	async getEntries() {
		return Object.fromEntries(
			await Promise.all(
				Object.entries(this.#entries).map(async ([key, value]) => [
					key,
					await getExpression(value),
				]),
			),
		);
	}
}

export type FileArgs = {
	blob: Hash;
	executable?: boolean;
};

export class File {
	#tangram = "file";
	blob;
	executable;

	constructor({ blob, executable }: FileArgs) {
		this.blob = blob;
		this.executable = executable ?? false;
	}

	static fromJson(file: Tangram.File): File {
		let blob = new Hash(file.blob);
		let executable = file.executable;
		return new File({ blob, executable });
	}

	async toJson(): Promise<Tangram.Expression> {
		return {
			type: ExpressionType.File,
			value: {
				blob: this.blob.toString(),
				executable: this.executable,
			},
		};
	}

	async getBlob(): Promise<Uint8Array> {
		return await getBlob(this.blob);
	}
}

export class Symlink {
	#tangram = "symlink";
	target: string;

	constructor(target: string) {
		this.target = target;
	}

	static fromJson(symlink: Tangram.Symlink): Symlink {
		return new Symlink(symlink.target);
	}

	async toJson(): Promise<Tangram.Expression> {
		return {
			type: ExpressionType.Symlink,
			value: {
				target: this.target,
			},
		};
	}
}

export type DependencyArgs = {
	artifact: HashOrExpression<Artifact>;
	path?: string | null;
};

export class Dependency {
	#tangram = "dependency";
	artifact: HashOrExpression<Artifact>;
	path: string | null;

	constructor({ artifact, path }: DependencyArgs) {
		this.artifact = artifact;
		this.path = path ?? null;
	}

	static fromJson(dependency: Tangram.Dependency): Dependency {
		return new Dependency({
			artifact: new Hash(dependency.artifact),
			path: dependency.path,
		});
	}

	async toJson(): Promise<Tangram.Expression> {
		let artifact = await addExpression(this.artifact);
		return {
			type: ExpressionType.Dependency,
			value: {
				artifact: artifact.toString(),
				path: this.path,
			},
		};
	}

	async getArtifact() {
		return await getExpression(this.artifact);
	}
}

type PackageArgs = {
	source: HashOrExpression<Artifact>;
	dependencies: { [name: string]: HashOrExpression<Package> };
};

export class Package {
	#tangram = "package";
	source: HashOrExpression<Artifact>;
	dependencies: { [key: string]: HashOrExpression<Package> };

	constructor({ source, dependencies }: PackageArgs) {
		this.source = source;
		this.dependencies = dependencies;
	}

	static fromJson(_package: Tangram.Package): Package {
		let source = new Hash(_package.source);
		let dependencies = Object.fromEntries(
			Object.entries(_package.dependencies).map(([key, value]) => [
				key,
				new Hash(value),
			]),
		);
		return new Package({
			source,
			dependencies,
		});
	}

	async toJson(): Promise<Tangram.Expression> {
		let source = await addExpression(this.source);
		let dependencies = Object.fromEntries(
			await Promise.all(
				Object.entries(this.dependencies).map(async ([key, value]) => [
					key,
					(await addExpression(value)).toString(),
				]),
			),
		);
		return {
			type: ExpressionType.Package,
			value: {
				source: source.toString(),
				dependencies,
			},
		};
	}

	async getSource(): Promise<Expression<Artifact>> {
		return await getExpression(this.source);
	}

	async getDependencies() {
		return Object.fromEntries(
			await Promise.all(
				Object.entries(this.dependencies).map(async ([key, value]) => [
					key,
					await getExpression(value),
				]),
			),
		);
	}
}

export class Template {
	#tangram = "template";
	components: Array<HashOrExpression<string | Artifact | Template>>;

	constructor(
		components: Array<HashOrExpression<string | Artifact | Template>>,
	) {
		this.components = components;
	}

	static fromJson(template: Tangram.Template): Template {
		return new Template(template.components.map((string) => new Hash(string)));
	}

	async toJson(): Promise<Tangram.Expression> {
		let components = await Promise.all(
			this.components.map(async (component) =>
				(await addExpression(component)).toString(),
			),
		);
		return {
			type: ExpressionType.Template,
			value: {
				components,
			},
		};
	}

	async getComponents(): Promise<
		Array<HashOrExpression<string | Artifact | Template>>
	> {
		return await Promise.all(
			this.components.map(async (component) => await getExpression(component)),
		);
	}
}

type JsArgs = {
	package: HashOrExpression<Package>;
	path: string;
	name: string;
	args: HashOrExpression<Array<AnyExpression>>;
};

export class Js<O extends AnyExpression> {
	#tangram = "js";
	package: HashOrExpression<Package>;
	path: string;
	name: string;
	args: HashOrExpression<Array<AnyExpression>>;

	constructor({ package: _package, path, name, args }: JsArgs) {
		this.package = _package;
		this.path = path;
		this.name = name;
		this.args = args;
	}

	static fromJson<O extends AnyExpression>(js: Tangram.Js): Js<O> {
		let _package = new Hash(js.package);
		let path = js.path;
		let name = js.name;
		let args = new Hash(js.args);
		return new Js({ package: _package, path, name, args });
	}

	async toJson(): Promise<Tangram.Expression> {
		let _package = await addExpression(this.package);
		let args = await addExpression(this.args);
		return {
			type: ExpressionType.Js,
			value: {
				package: _package.toString(),
				name: this.name,
				path: this.path,
				args: args.toString(),
			},
		};
	}

	async getPackage(): Promise<Expression<Package>> {
		return await getExpression(this.package);
	}

	async getArgs(): Promise<HashOrExpression<Array<AnyExpression>>> {
		return await getExpression(this.args);
	}
}

export type FetchArgs = {
	digest?: Digest | null;
	unpack?: boolean | null;
	url: string;
};

export class Fetch {
	#tangram = "fetch";
	url: string;
	digest: Digest | null;
	unpack: boolean;

	constructor({ url, digest, unpack }: FetchArgs) {
		this.url = url;
		this.digest = digest ?? null;
		this.unpack = unpack ?? false;
	}

	static fromJson(fetch: Tangram.Fetch) {
		return new Fetch({
			url: fetch.url,
			digest: fetch.digest,
			unpack: fetch.unpack,
		});
	}

	async toJson(): Promise<Tangram.Expression> {
		return {
			type: ExpressionType.Fetch,
			value: {
				url: this.url,
				digest: this.digest,
				unpack: this.unpack,
			},
		};
	}
}

type ProcessArgs = {
	system: System;
	env: HashOrExpression<{
		[key: string]: Expression<string | Artifact | Template>;
	}>;
	command: HashOrExpression<Artifact | Template>;
	args: HashOrExpression<Array<Expression<string | Artifact | Template>>>;
	digest?: Digest | null;
	unsafe?: boolean | null;
	network?: boolean | null;
};

export class Process {
	#tangram = "process";
	system: System;
	env: HashOrExpression<{ [key: string]: AnyExpression }>;
	command: HashOrExpression<Artifact | Template>;
	args: HashOrExpression<Array<AnyExpression>>;
	digest: Digest | null;
	unsafe: boolean | null;
	network: boolean | null;

	constructor(args: ProcessArgs) {
		this.system = args.system;
		this.env = args.env;
		this.command = args.command;
		this.args = args.args;
		this.digest = args.digest ?? null;
		this.network = args.network ?? null;
		this.unsafe = args.unsafe ?? null;
	}

	static fromJson(process: Tangram.Process): Process {
		let system = process.system;
		let env = new Hash(process.env);
		let command = new Hash(process.command);
		let args = new Hash(process.args);
		let digest = process.digest;
		let network = process.network;
		let unsafe = process.unsafe;
		return new Process({
			system,
			env,
			command,
			args,
			digest,
			unsafe,
			network,
		});
	}

	async toJson(): Promise<Tangram.Expression> {
		let env = await addExpression(this.env);
		let command = await addExpression(this.command);
		let args = await addExpression(this.args);
		return {
			type: ExpressionType.Process,
			value: {
				system: this.system,
				env: env.toString(),
				command: command.toString(),
				args: args.toString(),
				digest: this.digest,
				network: this.network,
				unsafe: this.unsafe,
			},
		};
	}

	async getEnv(): Promise<
		HashOrExpression<{
			[key: string]: AnyExpression;
		}>
	> {
		return await getExpression(this.env);
	}

	async getCommand(): Promise<HashOrExpression<Artifact | Template>> {
		return await getExpression(this.command);
	}

	async getArgs(): Promise<
		HashOrExpression<Array<Expression<string | Artifact | Template>>>
	> {
		return (await getExpression(this.args)) as any;
	}
}

type TargetArgs = {
	package: HashOrExpression<Package>;
	name: string;
	args: HashOrExpression<Array<AnyExpression>>;
};

export class Target<O extends AnyExpression> {
	#tangram = "target";
	package: HashOrExpression<Package>;
	name: string;
	args: HashOrExpression<Array<AnyExpression>>;

	constructor(args: TargetArgs) {
		this.package = args.package;
		this.name = args.name;
		this.args = args.args;
	}

	static fromJson<O extends AnyExpression>(json: Tangram.Target): Target<O> {
		return new Target({
			package: new Hash(json.package),
			name: json.name,
			args: new Hash(json.args),
		});
	}

	async toJson(): Promise<Tangram.Expression> {
		let _package = await addExpression(this.package);
		let args = await addExpression(this.args);
		return {
			type: ExpressionType.Target,
			value: {
				package: _package.toString(),
				name: this.name,
				args: args.toString(),
			},
		};
	}

	async getPackage(): Promise<Expression<Package>> {
		return await getExpression(this.package);
	}

	async getArgs(): Promise<Expression<Array<AnyExpression>>> {
		return await getExpression(this.args);
	}
}

export let template = (
	strings: TemplateStringsArray,
	...placeholders: Array<Expression<string | Artifact | Template>>
): Template => {
	let components: Array<string | Expression<string | Artifact | Template>> = [];
	for (let i = 0; i < strings.length - 1; i++) {
		let string = strings[i];
		let placeholder = placeholders[i];
		components.push(string);
		components.push(placeholder);
	}
	components.push(strings[strings.length - 1]);
	return new Template(components);
};

export let fromJson = async (
	expression: Tangram.Expression,
): Promise<AnyExpression> => {
	switch (expression.type) {
		case ExpressionType.Null: {
			return expression.value;
		}
		case ExpressionType.Bool: {
			return expression.value;
		}
		case ExpressionType.Number: {
			return expression.value;
		}
		case ExpressionType.String: {
			return expression.value;
		}
		case ExpressionType.Artifact: {
			return Artifact.fromJson(expression.value);
		}
		case ExpressionType.Directory: {
			return Directory.fromJson(expression.value);
		}
		case ExpressionType.File: {
			return File.fromJson(expression.value);
		}
		case ExpressionType.Symlink: {
			return Symlink.fromJson(expression.value);
		}
		case ExpressionType.Dependency: {
			return Dependency.fromJson(expression.value);
		}
		case ExpressionType.Package: {
			return Package.fromJson(expression.value);
		}
		case ExpressionType.Template: {
			return Template.fromJson(expression.value);
		}
		case ExpressionType.Js: {
			return Js.fromJson(expression.value);
		}
		case ExpressionType.Fetch: {
			return Fetch.fromJson(expression.value);
		}
		case ExpressionType.Process: {
			return Process.fromJson(expression.value);
		}
		case ExpressionType.Target: {
			return Target.fromJson(expression.value);
		}
		case ExpressionType.Array: {
			return await Promise.all(
				expression.value.map(
					async (value) => await getExpression(new Hash(value)),
				),
			);
		}
		case ExpressionType.Map: {
			return Object.fromEntries(
				await Promise.all(
					Object.entries(expression.value).map(async ([key, value]) => [
						key,
						await getExpression(new Hash(value)),
					]),
				),
			);
		}
		default: {
			throw new Error(`Invalid expression type "${expression.type}".`);
		}
	}
};

export let toJson = async (
	expression: AnyExpression,
): Promise<Tangram.Expression> => {
	if (expression === null || expression === undefined) {
		return {
			type: ExpressionType.Null,
			value: expression,
		};
	} else if (typeof expression === "boolean") {
		return {
			type: ExpressionType.Bool,
			value: expression,
		};
	} else if (typeof expression === "number") {
		return {
			type: ExpressionType.Number,
			value: expression,
		};
	} else if (typeof expression === "string") {
		return {
			type: ExpressionType.String,
			value: expression,
		};
	} else if (expression instanceof Artifact) {
		return await expression.toJson();
	} else if (expression instanceof Directory) {
		return await expression.toJson();
	} else if (expression instanceof File) {
		return await expression.toJson();
	} else if (expression instanceof Symlink) {
		return await expression.toJson();
	} else if (expression instanceof Dependency) {
		return await expression.toJson();
	} else if (expression instanceof Package) {
		return await expression.toJson();
	} else if (expression instanceof Template) {
		return await expression.toJson();
	} else if (expression instanceof Js) {
		return await expression.toJson();
	} else if (expression instanceof Fetch) {
		return await expression.toJson();
	} else if (expression instanceof Process) {
		return await expression.toJson();
	} else if (expression instanceof Target) {
		return await expression.toJson();
	} else if (Array.isArray(expression)) {
		let value = await Promise.all(
			expression.map(async (value) => {
				return (await addExpression(value)).toString();
			}),
		);
		return {
			type: ExpressionType.Array,
			value,
		};
	} else if (typeof expression === "object") {
		let value = Object.fromEntries(
			await Promise.all(
				Object.entries(expression).map(async ([key, value]) => [
					key,
					(await addExpression(value)).toString(),
				]),
			),
		);
		return {
			type: ExpressionType.Map,
			value,
		};
	} else {
		throw new Error("Attempted to hash a value that is not an expression.");
	}
};

export let addBlob = async (bytes: Uint8Array): Promise<Hash> => {
	return new Hash(await Tangram.syscall(Tangram.Syscall.AddBlob, bytes));
};

export let getBlob = async (hash: Hash): Promise<Uint8Array> => {
	return await Tangram.syscall(Tangram.Syscall.GetBlob, hash.toString());
};

export let addExpression = async <E extends AnyExpression>(
	hashOrExpression: Hash<E> | E,
): Promise<Hash<E>> => {
	if (hashOrExpression instanceof Hash) {
		return hashOrExpression;
	} else {
		return new Hash(
			await Tangram.syscall(
				Tangram.Syscall.AddExpression,
				await toJson(hashOrExpression),
			),
		);
	}
};

export let getExpression = async <E extends AnyExpression>(
	hashOrExpression: Hash<E> | E,
): Promise<E> => {
	if (hashOrExpression instanceof Hash) {
		return (await fromJson(
			await Tangram.syscall(
				Tangram.Syscall.GetExpression,
				hashOrExpression.toString(),
			),
		)) as E;
	} else {
		return hashOrExpression;
	}
};

export let evaluate = <E extends AnyExpression>(
	expression: E,
): Promise<OutputForExpression<E>> => {
	return (async () => {
		let hash = await addExpression(expression);
		let outputHash = new Hash(
			await Tangram.syscall(Tangram.Syscall.Evaluate, hash.toString()),
		);
		let output = await getExpression(outputHash);
		return output;
	})() as any;
};

export let source = async (url: string | URL): Promise<Artifact> => {
	let hash = new Hash(new URL(url).hostname);
	let _package = await getExpression(hash);
	if (!(_package instanceof Package)) {
		throw new Error("Expected package.");
	}
	let source = await _package.getSource();
	if (!(source instanceof Artifact)) {
		throw new Error("Expected artifact.");
	}
	return source;
};

export enum SerializationFormat {
	Toml = "toml",
}

export let deserialize = <T>(
	format: SerializationFormat,
	contents: string,
): T => {
	return Tangram.syscall(Tangram.Syscall.Deserialize, format, contents);
};
