/* eslint no-restricted-syntax: off */

declare module Tangram {
  export function syscall(
    syscall: Syscall.AddBlob,
    bytes: string
  ): Promise<Hash>;

  export function syscall(
    syscall: Syscall.GetBlob,
    hash: Hash
  ): Promise<Uint8Array>;

  export function syscall(
    syscall: Syscall.AddExpression,
    expression: Expression
  ): Hash;

  export function syscall(
    syscall: Syscall.GetExpression,
    hash: Hash
  ): Promise<Expression>;

  export function syscall(syscall: Syscall.Evaluate, hash: Hash): Promise<Hash>;

  export enum Syscall {
    AddBlob = "add_blob",
    GetBlob = "get_blob",
    AddExpression = "add_expression",
    GetExpression = "get_expression",
    Evaluate = "evaluate",
  }

  export type Hash = string;

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
    Path = "path",
    Template = "template",
    Fetch = "fetch",
    Process = "process",
    Target = "target",
    Array = "array",
    Map = "map",
  }

  export type Expression =
    | { type: ExpressionType.Null; value: null }
    | { type: ExpressionType.Bool; value: boolean }
    | { type: ExpressionType.Number; value: number }
    | { type: ExpressionType.String; value: string }
    | { type: ExpressionType.Artifact; value: Artifact }
    | { type: ExpressionType.Directory; value: Directory }
    | { type: ExpressionType.File; value: File }
    | { type: ExpressionType.Symlink; value: Symlink }
    | { type: ExpressionType.Dependency; value: Dependency }
    | { type: ExpressionType.Path; value: Path }
    | { type: ExpressionType.Template; value: Template }
    | { type: ExpressionType.Fetch; value: Fetch }
    | { type: ExpressionType.Process; value: Process }
    | { type: ExpressionType.Target; value: Target }
    | { type: ExpressionType.Array; value: _Array }
    | { type: ExpressionType.Map; value: _Map };

  export type Artifact = {
    hash: Hash;
  };

  export type Directory = {
    entries: { [key: string]: Hash };
  };

  export type File = {
    executable?: boolean;
    hash: Hash;
  };

  export type Symlink = {
    target: string;
  };

  export type Dependency = {
    artifact: Hash;
  };

  export type Path = {
    artifact: Hash;
    path?: string;
  };

  export type Template = {
    components: Array<Hash>;
  };

  export type Fetch = {
    hash?: Hash;
    unpack?: boolean;
    url: string;
  };

  export enum System {
    Amd64Linux = "amd64_linux",
    Amd64Macos = "amd64_macos",
    Arm64Linux = "arm64_linux",
    Arm64Macos = "arm64_macos",
    Js = "js",
  }

  export type Process =
    | ({ system: System.Amd64Linux } & UnixProcess)
    | ({ system: System.Amd64Macos } & UnixProcess)
    | ({ system: System.Arm64Linux } & UnixProcess)
    | ({ system: System.Arm64Macos } & UnixProcess)
    | ({ system: System.Js } & JsProcess);

  export type UnixProcess = {
    args: Hash;
    command: Hash;
    env: Hash;
    outputs: { [key: string]: UnixProcessOutput };
  };

  export type UnixProcessOutput = {
    dependencies: { [key: string]: Hash };
  };

  export type JsProcess = {
    args: Hash;
    export: string;
    lockfile?: any;
    module: Hash;
  };

  export type Target = {
    args: Hash;
    lockfile?: any;
    name: string;
    package: Hash;
  };

  export type _Array = Array<Hash>;

  export type _Map = { [key: string]: Hash };
}

export enum System {
  Amd64Linux = "amd64_linux",
  Amd64Macos = "amd64_macos",
  Arm64Linux = "arm64_linux",
  Arm64Macos = "arm64_macos",
  Js = "js",
}

export type MaybePromise<T> = T | Promise<T>;

export class Hash<T extends Expression | unknown = unknown> {
  hash: string;

  constructor(hash: string) {
    this.hash = hash;
  }
}

export type HashOr<T extends Expression> = Hash<T> | T;

export type Expression =
  | null
  | boolean
  | number
  | string
  | Artifact
  | Directory
  | File
  | Symlink
  | Dependency
  | Path
  | Template
  | Fetch
  | Process
  | Target
  | Array<HashOr<Expression>>
  | { [key: string]: HashOr<Expression> };

export class Artifact {
  #hash: Hash<Artifact> | null;
  #artifact: Tangram.Artifact | null;
  #expression: HashOr<Expression>;

  constructor(expression: HashOr<Expression>) {
    this.#expression = expression;
    this.#hash = null;
    this.#artifact = null;
  }

  public async hash(): Promise<Hash<Artifact>> {
    if (this.#hash === null) {
      this.#artifact = { hash: (await hash(this.#expression)).hash };
      this.#hash = new Hash(
        await Tangram.syscall(Tangram.Syscall.AddExpression, {
          type: Tangram.ExpressionType.Artifact,
          value: this.#artifact,
        })
      );
    }
    return this.#hash;
  }

  public static async fromHash(hash: Hash): Promise<Artifact> {
    let expression = await Tangram.syscall(
      Tangram.Syscall.GetExpression,
      hash.hash
    );
    if (expression.type !== Tangram.ExpressionType.Artifact) {
      throw new Error("Expected an artifact.");
    }
    return Artifact.fromExpression(hash, expression.value);
  }

  set artifact(artifact: Tangram.Artifact) {
    this.#artifact = artifact;
  }

  private async setHash(hash: Hash) {
    this.#hash = hash;
  }

  private async setArtifact(artifact: Tangram.Artifact) {
    this.#artifact = artifact;
  }

  public static fromExpression(hash: Hash, artifact: Tangram.Artifact) {
    let newArtifact: Artifact = new Artifact(artifact.hash);
    newArtifact.setHash(hash);
    newArtifact.setArtifact(artifact);
    return newArtifact;
  }
}

export class Directory {
  #hash: Hash<Directory> | null;
  #directory: Tangram.Directory | null;
  #entries: { [key: string]: HashOr<Expression> };

  constructor(entries: { [key: string]: HashOr<Expression> }) {
    this.#entries = entries;
    this.#hash = null;
    this.#directory = null;
  }

  public async hash(): Promise<Hash<Directory>> {
    let entries = await Promise.all(
      Object.entries(this.#entries).map(async ([key, value]) => [
        key,
        (await hash(value)).hash,
      ])
    );
    this.#directory = {
      entries: Object.fromEntries(entries),
    };
    this.#hash = new Hash(
      Tangram.syscall(Tangram.Syscall.AddExpression, {
        type: Tangram.ExpressionType.Directory,
        value: this.#directory,
      })
    );
    return this.#hash;
  }

  private setHash(hash: Hash) {
    this.#hash = hash;
  }

  private setDirectory(directory: Tangram.Directory) {
    this.#directory = directory;
  }

  public static fromExpression(hash: Hash, expression: Tangram.Directory) {
    let entries = Directory.entriesFromDirectory(expression);
    let directory: Directory = new Directory(entries);
    directory.setHash(hash);
    directory.setDirectory(expression);
    return directory;
  }

  private async getDirectory() {
    await this.hash();
    if (this.#directory === null) {
      throw new Error("unreachable");
    }
    return this.#directory;
  }

  private static entriesFromDirectory(directory: Tangram.Directory): {
    [key: string]: Hash;
  } {
    return Object.fromEntries(
      Object.entries(directory.entries).map(([key, value]) => [
        key,
        new Hash(value),
      ])
    );
  }

  async getEntries(): Promise<{ [key: string]: Hash }> {
    let directory = await this.getDirectory();
    return Directory.entriesFromDirectory(directory);
  }
}

export class File {
  #hash: Hash<File> | null;
  #file: Tangram.File;

  constructor(blob_hash: Hash, executable?: boolean) {
    this.#hash = null;
    this.#file = { executable, hash: blob_hash.hash };
  }

  public async hash(): Promise<Hash<File>> {
    this.#hash = new Hash(
      Tangram.syscall(Tangram.Syscall.AddExpression, {
        type: Tangram.ExpressionType.File,
        value: this.#file,
      })
    );
    return this.#hash;
  }

  private setHash(hash: Hash) {
    this.#hash = hash;
  }

  private setFile(expression: Tangram.File) {
    this.#file = expression;
  }

  public static fromExpression(hash: Hash, expression: Tangram.File) {
    let file: File = new File(new Hash(expression.hash), expression.executable);
    file.setHash(hash);
    file.setFile(expression);
    return file;
  }

  get executable(): boolean | undefined {
    return this.#file.executable;
  }

  get blobHash(): Hash<Blob> {
    return new Hash(this.#file.hash);
  }
}

export class Symlink {
  #hash: Hash<Symlink> | null;
  #symlink: Tangram.Symlink;

  constructor(target: string) {
    this.#symlink = { target };
    this.#hash = null;
  }

  public async hash(): Promise<Hash<Symlink>> {
    this.#hash = new Hash(
      Tangram.syscall(Tangram.Syscall.AddExpression, {
        type: Tangram.ExpressionType.Symlink,
        value: this.#symlink,
      })
    );
    return this.#hash;
  }

  private setHash(hash: Hash) {
    this.#hash = hash;
  }

  private setSymlink(expression: Tangram.Symlink) {
    this.#symlink = expression;
  }

  public static fromExpression(hash: Hash, expression: Tangram.Symlink) {
    let symlink: Symlink = new Symlink(expression.target);
    symlink.setHash(hash);
    symlink.setSymlink(expression);
    return symlink;
  }

  get target() {
    return this.#symlink.target;
  }
}

export class Dependency {
  #hash: Hash<Dependency> | null;
  #dependency: Tangram.Dependency | null;
  #artifact: HashOr<Expression>;

  constructor(artifact: HashOr<Artifact>) {
    this.#dependency = null;
    this.#hash = null;
    this.#artifact = artifact;
  }

  public async hash(): Promise<Hash<Dependency>> {
    this.#dependency = { artifact: (await hash(this.#artifact)).hash };
    this.#hash = new Hash(
      Tangram.syscall(Tangram.Syscall.AddExpression, {
        type: Tangram.ExpressionType.Dependency,
        value: this.#dependency,
      })
    );
    return this.#hash;
  }

  private setHash(hash: Hash) {
    this.#hash = hash;
  }

  private setDependency(dependency: Tangram.Dependency) {
    this.#dependency = dependency;
  }

  public static fromExpression(hash: Hash, expression: Tangram.Dependency) {
    let dependency: Dependency = new Dependency(new Hash(expression.artifact));
    dependency.setHash(hash);
    dependency.setDependency(expression);
    return dependency;
  }

  public static async fromHash(hash: Hash) {
    let expression = await Tangram.syscall(
      Tangram.Syscall.GetExpression,
      hash.hash
    );
    if (expression.type !== Tangram.ExpressionType.Dependency) {
      throw new Error("Expected a dependency.");
    }
    return Dependency.fromExpression(hash, expression.value);
  }

  public async artifact() {
    await this.hash();
    if (this.#dependency === null) {
      throw new Error("unreachable");
    }
    return Artifact.fromHash(new Hash(this.#dependency.artifact));
  }
}

export type PathArgs = {
  artifact: Expression;
  path?: string;
};

export class Path {
  #hash: Hash<Path> | null;
  #path: Tangram.Path | null;
  #artifact: HashOr<Expression>;
  #subpath: string | undefined;

  constructor(artifact: Expression, path?: string) {
    this.#path = null;
    this.#hash = null;
    this.#artifact = artifact;
    this.#subpath = path;
  }

  public async hash(): Promise<Hash<Path>> {
    this.#path = {
      artifact: (await hash(this.#artifact)).hash,
      path: this.#subpath,
    };
    this.#hash = new Hash(
      Tangram.syscall(Tangram.Syscall.AddExpression, {
        type: Tangram.ExpressionType.Path,
        value: this.#path,
      })
    );
    return this.#hash;
  }

  private setHash(hash: Hash) {
    this.#hash = hash;
  }

  private setPath(path: Tangram.Path) {
    this.#path = path;
  }

  public static fromExpression(hash: Hash, expression: Tangram.Path) {
    let path: Path = new Path(expression.artifact, expression.path);
    path.setHash(hash);
    path.setPath(expression);
    return path;
  }

  public static async fromHash(hash: Hash) {
    let expression = await Tangram.syscall(
      Tangram.Syscall.GetExpression,
      hash.hash
    );
    if (expression.type !== Tangram.ExpressionType.Path) {
      throw new Error("Expected a path.");
    }
    return Path.fromExpression(hash, expression.value);
  }

  // This does not need to be async because we are storing it on the object.
  public async path() {
    await this.hash();
    if (this.#path === null) {
      throw new Error("unreachable");
    }
    return this.#path.path;
  }
}

export class Template {
  #hash: Hash<Template> | null;
  #template: Tangram.Template | null;
  #components: Array<HashOr<Expression>>;

  constructor(components: Array<HashOr<Expression>>) {
    this.#components = components;
    this.#template = null;
    this.#hash = null;
  }

  public async hash(): Promise<Hash<Template>> {
    this.#template = {
      components: await Promise.all(
        this.#components.map(async (component) => (await hash(component)).hash)
      ),
    };
    this.#hash = new Hash(
      Tangram.syscall(Tangram.Syscall.AddExpression, {
        type: Tangram.ExpressionType.Template,
        value: this.#template,
      })
    );
    return this.#hash;
  }

  private setHash(hash: Hash) {
    this.#hash = hash;
  }

  private setTemplate(expression: Tangram.Template) {
    this.#template = expression;
  }

  public static fromExpression(hash: Hash, expression: Tangram.Template) {
    let template: Template = new Template(expression.components);
    template.setHash(hash);
    template.setTemplate(expression);
    return template;
  }

  public static async fromHash(hash: Hash) {
    let expression = await Tangram.syscall(
      Tangram.Syscall.GetExpression,
      hash.hash
    );
    if (expression.type !== Tangram.ExpressionType.Template) {
      throw new Error("Expected a template.");
    }
    return Template.fromExpression(hash, expression.value);
  }

  public async components() {
    await this.hash();
    if (this.#template === null) {
      throw new Error("unreachable");
    }
    return this.#template.components.map((v) => new Hash(v));
  }
}

export type FetchArgs = {
  hash?: Hash;
  unpack?: boolean;
  url: string;
};

export class Fetch {
  #hash: Hash<Fetch> | null;
  #fetch: Tangram.Fetch | null;
  #unpack: boolean | undefined;
  #url: string;
  #contentHash: Hash | undefined;

  constructor(args: FetchArgs) {
    this.#hash = null;
    this.#fetch = null;
    this.#url = args.url;
    this.#unpack = args.unpack;
    this.#contentHash = args.hash;
  }

  public async hash(): Promise<Hash<Fetch>> {
    this.#fetch = {
      hash: this.#contentHash?.hash,
      unpack: this.#unpack,
      url: this.#url,
    };
    this.#hash = new Hash(
      Tangram.syscall(Tangram.Syscall.AddExpression, {
        type: Tangram.ExpressionType.Fetch,
        value: this.#fetch,
      })
    );
    return this.#hash;
  }

  private setHash(hash: Hash) {
    this.#hash = hash;
  }

  private setFetch(fetch: Tangram.Fetch) {
    this.#fetch = fetch;
  }

  public static fromExpression(hash: Hash, expression: Tangram.Fetch) {
    let fetch: Fetch = new Fetch({
      hash: expression.hash ? new Hash(expression.hash) : undefined,
      unpack: expression.unpack,
      url: expression.url,
    });
    fetch.setHash(hash);
    fetch.setFetch(expression);
    return fetch;
  }

  public static async fromHash(hash: Hash) {
    let expression = await Tangram.syscall(
      Tangram.Syscall.GetExpression,
      hash.hash
    );
    if (expression.type !== Tangram.ExpressionType.Fetch) {
      throw new Error("Expected a fetch.");
    }
    return Fetch.fromExpression(hash, expression.value);
  }
}

export type ProcessArgs =
  | ({ system: System.Amd64Linux } & UnixProcessArgs)
  | ({ system: System.Amd64Macos } & UnixProcessArgs)
  | ({ system: System.Arm64Linux } & UnixProcessArgs)
  | ({ system: System.Arm64Macos } & UnixProcessArgs)
  | ({ system: System.Js } & JsProcessArgs);

export type UnixProcessArgs = {
  args: HashOr<Expression>;
  command: HashOr<Template>;
  env: HashOr<Expression>;
  outputs: { [key: string]: UnixProcessOutput };
  system: System;
};

export type UnixProcessOutput = {
  dependencies?: { [key: string]: HashOr<Expression> };
};

export type JsProcessArgs = {};

export class Process {
  #hash: Hash<Process> | null;
  #args: ProcessArgs | null;
  #process: Tangram.Process | null;

  constructor(args: ProcessArgs) {
    this.#args = args;
    this.#hash = null;
    this.#process = null;
  }

  public async hash(): Promise<Hash<Process>> {
    if (this.#hash !== null) {
      return this.#hash;
    }
    if (this.#args === null) {
      throw new Error();
    }
    if (this.#args.system === System.Js) {
      throw new Error();
    }
    let mapDependencies = async (dependencies?: {
      [key: string]: HashOr<Expression>;
    }): Promise<{ [key: string]: Hash } | undefined> => {
      if (!dependencies) {
        return dependencies;
      }
      return Object.fromEntries(
        await Promise.all(
          Object.entries(dependencies).map(async ([key, value]) => [
            key,
            (await hash(value)).hash,
          ])
        )
      );
    };
    let process = {
      args: (await hash(this.#args.args)).hash,
      command: (await hash(this.#args.command)).hash,
      env: (await hash(this.#args.env)).hash,
      outputs: Object.fromEntries(
        await Promise.all(
          Object.entries(this.#args.outputs).map(async ([key, value]) => [
            key,
            {
              dependencies: mapDependencies(value.dependencies),
            },
          ])
        )
      ),
      system: this.#args.system,
    };
    this.#process = process;
    this.#hash = new Hash(
      Tangram.syscall(Tangram.Syscall.AddExpression, {
        type: Tangram.ExpressionType.Process,
        value: process,
      })
    );
    return this.#hash;
  }

  private setHash(hash: Hash) {
    this.#hash = hash;
  }

  private setProcess(process: Tangram.Process) {
    this.#process = process;
  }

  public static fromExpression(hash: Hash, expression: Tangram.Process) {
    if (expression.system === System.Js) {
      throw new Error();
    }
    let mapDependencies = (dependencies?: {
      [key: string]: Tangram.Hash;
    }): { [key: string]: Hash } | undefined => {
      if (!dependencies) {
        return dependencies;
      }
      return Object.fromEntries(
        Object.entries(dependencies).map(([key, value]) => [
          key,
          new Hash(value),
        ])
      );
    };
    let process_args: ProcessArgs = {
      args: new Hash(expression.args),
      command: new Hash(expression.command),
      env: new Hash(expression.env),
      outputs: Object.fromEntries(
        Object.entries(expression.outputs).map(([key, value]) => [
          key,
          { dependencies: mapDependencies(value.dependencies) },
        ])
      ),
      system: expression.system,
    };
    let process: Process = new Process(process_args);
    process.setHash(hash);
    process.setProcess(expression);
    return process;
  }

  public static async fromHash(hash: Hash) {
    let expression = await Tangram.syscall(
      Tangram.Syscall.GetExpression,
      hash.hash
    );
    if (expression.type !== Tangram.ExpressionType.Process) {
      throw new Error("Expected a process.");
    }
    return Process.fromExpression(hash, expression.value);
  }
}

export type TargetArgs = {
  args: HashOr<Array<HashOr<Expression>>>;
  lockfile: any;
  name: string;
  package: HashOr<Artifact>;
};

export class Target {
  #hash: Hash<Target> | null;
  #target: Tangram.Target | null;
  #args: TargetArgs;

  constructor(args: TargetArgs) {
    this.#target = null;
    this.#hash = null;
    this.#args = args;
  }

  public async hash(): Promise<Hash<Target>> {
    this.#target = {
      args: (await hash(this.#args.args)).hash,
      lockfile: this.#args.lockfile,
      name: this.#args.name,
      package: (await hash(this.#args.package)).hash,
    };
    this.#hash = new Hash(
      Tangram.syscall(Tangram.Syscall.AddExpression, {
        type: Tangram.ExpressionType.Target,
        value: this.#target,
      })
    );
    return this.#hash;
  }

  private setHash(hash: Hash) {
    this.#hash = hash;
  }

  private setTarget(target: Tangram.Target) {
    this.#target = target;
  }

  public static fromExpression(hash: Hash, expression: Tangram.Target) {
    let target: Target = new Target({
      args: new Hash(expression.args),
      lockfile: expression.lockfile,
      name: expression.name,
      package: new Hash(expression.package),
    });
    target.setHash(hash);
    target.setTarget(expression);
    return target;
  }

  public static async fromHash(hash: Hash) {
    let expression = await Tangram.syscall(
      Tangram.Syscall.GetExpression,
      hash.hash
    );
    if (expression.type !== Tangram.ExpressionType.Target) {
      throw new Error("Expected a target.");
    }
    return Target.fromExpression(hash, expression.value);
  }
}

export let hash = async <T extends Expression>(
  value: HashOr<T>
): Promise<Hash<T>> => {
  if (value instanceof Hash) {
    return value;
  } else if (value == null) {
    return new Hash(
      Tangram.syscall(Tangram.Syscall.AddExpression, {
        type: Tangram.ExpressionType.Null,
        value,
      })
    );
  } else if (typeof value == "boolean") {
    return new Hash(
      Tangram.syscall(Tangram.Syscall.AddExpression, {
        type: Tangram.ExpressionType.Bool,
        value,
      })
    );
  } else if (typeof value == "number") {
    return new Hash(
      Tangram.syscall(Tangram.Syscall.AddExpression, {
        type: Tangram.ExpressionType.Number,
        value,
      })
    );
  } else if (typeof value == "string") {
    return new Hash(
      Tangram.syscall(Tangram.Syscall.AddExpression, {
        type: Tangram.ExpressionType.String,
        value,
      })
    );
  } else if (value instanceof Artifact) {
    return await value.hash();
  } else if (value instanceof Directory) {
    return await value.hash();
  } else if (value instanceof File) {
    return await value.hash();
  } else if (value instanceof Symlink) {
    return await value.hash();
  } else if (value instanceof Dependency) {
    return await value.hash();
  } else if (value instanceof Path) {
    return await value.hash();
  } else if (value instanceof Template) {
    return await value.hash();
  } else if (value instanceof Fetch) {
    return await value.hash();
  } else if (value instanceof Process) {
    return await value.hash();
  } else if (value instanceof Target) {
    return await value.hash();
  } else if (value instanceof Array) {
    let array = await Promise.all(
      value.map(hash).map(async (hash) => (await hash).hash)
    );
    return new Hash(
      Tangram.syscall(Tangram.Syscall.AddExpression, {
        type: Tangram.ExpressionType.Array,
        value: array,
      })
    );
  } else if (typeof value == "object") {
    let map = Object.fromEntries(
      await Promise.all(
        Object.entries(value).map(async ([key, value]) => [
          key,
          (await hash(value)).hash,
        ])
      )
    );
    return new Hash(
      Tangram.syscall(Tangram.Syscall.AddExpression, {
        type: Tangram.ExpressionType.Map,
        value: map,
      })
    );
  } else {
    throw new Error("Attempted to hash a value that is not an expression.");
  }
};

export let blob = async (bytes: string): Promise<Hash> => {
  return new Hash(await Tangram.syscall(Tangram.Syscall.AddBlob, bytes));
};

export let evaluate = async (hash: Hash): Promise<Hash> => {
  return new Hash(await Tangram.syscall(Tangram.Syscall.Evaluate, hash.hash));
};

export let template = (
  strings: TemplateStringsArray,
  ...placeholders: Array<Expression>
): Template => {
  let components: Expression[] = [];
  for (let i = 0; i < strings.length - 1; i++) {
    let string = strings[i];
    let placeholder = placeholders[i]!;
    components.push(string);
    components.push(placeholder);
  }
  components.push(strings[strings.length - 1]);
  return new Template(components);
};

export async function getExpression(hash: Hash): Promise<Expression> {
  let expression = await Tangram.syscall(
    Tangram.Syscall.GetExpression,
    hash.hash
  );
  switch (expression.type) {
    case Tangram.ExpressionType.Null:
      return expression.value;
    case Tangram.ExpressionType.String:
      return expression.value;
    case Tangram.ExpressionType.Bool:
      return expression.value;
    case Tangram.ExpressionType.Number:
      return expression.value;
    case Tangram.ExpressionType.Artifact:
      return Artifact.fromExpression(hash, expression.value);
    case Tangram.ExpressionType.Directory:
      return Directory.fromExpression(hash, expression.value);
    case Tangram.ExpressionType.File:
      return File.fromExpression(hash, expression.value);
    case Tangram.ExpressionType.Symlink:
      return Symlink.fromExpression(hash, expression.value);
    case Tangram.ExpressionType.Dependency:
      return Dependency.fromExpression(hash, expression.value);
    case Tangram.ExpressionType.Path:
      return Path.fromExpression(hash, expression.value);
    case Tangram.ExpressionType.Template:
      return Template.fromExpression(hash, expression.value);
    case Tangram.ExpressionType.Fetch:
      return Fetch.fromExpression(hash, expression.value);
    case Tangram.ExpressionType.Process:
      return Process.fromExpression(hash, expression.value);
    case Tangram.ExpressionType.Target:
      return Target.fromExpression(hash, expression.value);
    default:
      throw new Error(`Invalid expression type "${expression.type}".`);
  }
}
