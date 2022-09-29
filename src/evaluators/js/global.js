/* eslint no-restricted-syntax: off */

let ExpressionType = {
  Null: "null",
  Bool: "bool",
  Number: "number",
  String: "string",
  Artifact: "artifact",
  Directory: "directory",
  File: "file",
  Symlink: "symlink",
  Dependency: "dependency",
  Template: "template",
  Js: "js",
  Fetch: "fetch",
  Process: "process",
  Target: "target",
  Array: "array",
  Map: "map",
};

let Syscall = {
  Console: "console",
  AddBlob: "add_blob",
  GetBlob: "get_blob",
  AddExpression: "add_expression",
  GetExpression: "get_expression",
  Evaluate: "evaluate",
};

let System = {
  Amd64Linux: "amd64_linux",
  Amd64Macos: "amd64_macos",
  Arm64Linux: "arm64_linux",
  Arm64Macos: "arm64_macos",
};

let syscall = (syscall, ...args) => {
  if (syscall !== Syscall.console) {
    console.log(syscall, ...args);
  }
  let opName = "op_tangram_" + syscall;
  switch (syscall) {
    case Syscall.Console:
      return Deno.core.opSync(opName, ...args);
    case Syscall.AddBlob:
      return Deno.core.opAsync(opName, ...args);
    case Syscall.GetBlob:
      return Deno.core.opAsync(opName, ...args);
    case Syscall.AddExpression:
      return Deno.core.opAsync(opName, ...args);
    case Syscall.GetExpression:
      return Deno.core.opAsync(opName, ...args);
    case Syscall.Evaluate:
      return Deno.core.opAsync(opName, ...args);
  }
};

class Hash {
  string;

  constructor(string) {
    this.string = string;
  }

  toString() {
    return this.string;
  }
}

class Artifact {
  root;

  constructor(root) {
    this.root = root;
  }

  static fromInternal(expression) {
    let root = new Hash(expression.value.root);
    return new Artifact(root);
  }

  async toInternal() {
    let root = await addExpression(this.root);
    return {
      type: ExpressionType.Artifact,
      value: {
        root: root.toString(),
      },
    };
  }
}

class Directory {
  entries;

  constructor(entries) {
    this.entries = entries;
  }

  static fromInternal(expression) {
    let entries = Object.fromEntries(
      Object.entries(expression.value.entries).map(([key, value]) => [
        key,
        new Hash(value),
      ])
    );
    return new Directory(entries);
  }

  async toInternal() {
    let entries = Object.fromEntries(
      await Promise.all(
        Object.entries(this.entries).map(async ([key, value]) => [
          key,
          (await addExpression(value)).toString(),
        ])
      )
    );
    return {
      type: ExpressionType.Directory,
      value: { entries },
    };
  }
}

class File {
  blob;
  executable;

  constructor(blob, executable) {
    this.blob = blob;
    this.executable = executable ?? false;
  }

  static fromInternal(expression) {
    let blob = new Hash(expression.value.blob);
    let executable = expression.value.executable;
    return new File(blob, executable);
  }

  async toInternal() {
    return {
      type: ExpressionType.File,
      value: {
        blob: this.blob.toString(),
        executable: this.executable,
      },
    };
  }
}

class Symlink {
  target;

  constructor(target) {
    this.target = target;
  }

  static fromInternal(expression) {
    return new Symlink(expression.value.target);
  }

  async toInternal() {
    return {
      type: ExpressionType.Symlink,
      value: {
        target: this.target,
      },
    };
  }
}

class Dependency {
  artifact;

  constructor(artifact) {
    this.artifact = artifact;
  }

  static fromInternal(expression) {
    return new Dependency(new Hash(expression.value.artifact));
  }

  async toInternal() {
    let artifact = await addExpression(this.artifact);
    return {
      type: ExpressionType.Dependency,
      value: {
        artifact: artifact.toString(),
      },
    };
  }
}

class Template {
  components;

  constructor(components) {
    this.components = components;
  }

  static fromInternal(expression) {
    return new Template(
      expression.value.components.map((string) => new Hash(string))
    );
  }

  async toInternal() {
    let components = await Promise.all(
      this.components.map(async (component) =>
        (await addExpression(component)).toString()
      )
    );
    return {
      type: ExpressionType.Template,
      value: {
        components,
      },
    };
  }
}

class Js {
  dependencies;
  artifact;
  path;
  name;
  args;

  constructor({ dependencies, artifact, path, name, args }) {
    this.dependencies = dependencies;
    this.artifact = artifact;
    this.path = path;
    this.name = name;
    this.args = args;
  }

  static fromInternal(expression) {
    let dependencies = Object.fromEntries(
      Object.entries(expression.value.dependencies).map(([key, value]) => [
        key,
        new Hash(value),
      ])
    );
    let artifact = new Hash(expression.value.artifact);
    let path = expression.value.path;
    let name = expression.value.name;
    let args = new Hash(expression.value.args);
    return new Js({ dependencies, artifact, path, name, args });
  }

  static async toInternal() {
    let artifact = await addExpression(this.artifact);
    let args = await addExpression(this.args);
    let dependencies = Object.fromEntries(
      Object.entries(this.dependencies).map(async ([key, value]) => [
        key,
        (await addExpression(value)).toString(),
      ])
    );
    return {
      type: ExpressionType.Js,
      value: {
        artifact: artifact.toString(),
        args: args.toString(),
        path: this.path,
        name: this.name,
        dependencies,
      },
    };
  }
}

class Fetch {
  url;
  hash;
  unpack;

  constructor({ url, hash, unpack }) {
    this.url = url;
    this.hash = hash;
    this.unpack = unpack;
  }

  static fromInternal(expression) {
    return new Fetch({
      url: expression.value.url,
      hash: expression.value.hash,
      unpack: expression.value.unpack,
    });
  }

  async toInternal() {
    return {
      type: ExpressionType.Fetch,
      value: {
        url: this.url,
        hash: this.hash,
        unpack: this.unpack,
      },
    };
  }
}

class Process {
  system;
  env;
  command;
  args;

  constructor({ system, env, command, args }) {
    this.system = system;
    this.env = env;
    this.command = command;
    this.args = args;
  }

  static fromInternal(expression) {
    let system = expression.value.system;
    let artifact = new Hash(expression.value.env);
    let command = new Hash(expression.value.command);
    let args = new Hash(expression.value.args);
    return new Process({ system, artifact, command, args });
  }

  static async toInternal() {
    let artifact = await addExpression(this.artifact);
    let command = await addExpression(this.command);
    let args = await addExpression(this.args);
    return {
      type: ExpressionType.Process,
      value: {
        system: this.system,
        artifact: artifact.toString(),
        command: command.toString(),
        args: args.toString(),
      },
    };
  }
}

class Target {
  package;
  name;
  args;

  constructor({ package, name, args }) {
    this.package = package;
    this.name = name;
    this.args = args;
  }

  static fromInternal(expression) {
    return new Target({
      package: new Hash(expression.value.package),
      name: expression.value.name,
      args: new Hash(expression.value.args),
    });
  }

  async toInternal() {
    let package = await addExpression(this.package);
    let args = await addExpression(this.args);
    return {
      type: ExpressionType.Target,
      value: {
        package: package.toString(),
        name: this.name,
        args: args.toString(),
      },
    };
  }
}

let fromInternal = (expression) => {
  switch (expression.type) {
    case ExpressionType.Null:
      return expression.value;
    case ExpressionType.Bool:
      return expression.value;
    case ExpressionType.Number:
      return expression.value;
    case ExpressionType.String:
      return expression.value;
    case ExpressionType.Artifact:
      return Artifact.fromInternal(expression);
    case ExpressionType.Directory:
      return Directory.fromInternal(expression);
    case ExpressionType.File:
      return File.fromInternal(expression);
    case ExpressionType.Symlink:
      return Symlink.fromInternal(expression);
    case ExpressionType.Dependency:
      return Dependency.fromInternal(expression);
    case ExpressionType.Template:
      return Template.fromInternal(expression);
    case ExpressionType.Js:
      return Js.fromInternal(expression);
    case ExpressionType.Fetch:
      return Fetch.fromInternal(expression);
    case ExpressionType.Process:
      return Process.fromInternal(expression);
    case ExpressionType.Target:
      return Target.fromInternal(expression);
    case ExpressionType.Array:
      return expression.value.map((value) => new Hash(value));
    case ExpressionType.Map:
      return Object.fromEntries(
        Object.entries(expression.value).map(([key, value]) => [
          key,
          new Hash(value),
        ])
      );
    default:
      throw new Error(`Invalid expression type "${expression.type}".`);
  }
};

let toInternal = async (expression) => {
  if (value === null) {
    return {
      type: ExpressionType.Null,
      value,
    };
  } else if (typeof value === "boolean") {
    return {
      type: ExpressionType.Bool,
      value,
    };
  } else if (typeof value === "number") {
    return {
      type: ExpressionType.Number,
      value,
    };
  } else if (typeof value === "string") {
    return {
      type: ExpressionType.String,
      value,
    };
  } else if (value instanceof Artifact) {
    return await value.toInternal();
  } else if (value instanceof Directory) {
    return await value.toInternal();
  } else if (value instanceof File) {
    return await value.toInternal();
  } else if (value instanceof Symlink) {
    return await value.toInternal();
  } else if (value instanceof Dependency) {
    return await value.toInternal();
  } else if (value instanceof Template) {
    return await value.toInternal();
  } else if (value instanceof Js) {
    return await value.toInternal();
  } else if (value instanceof Fetch) {
    return await value.toInternal();
  } else if (value instanceof Process) {
    return await value.toInternal();
  } else if (value instanceof Target) {
    return await value.toInternal();
  } else if (Array.isArray(value)) {
    let value = await Promise.all(value.map(addExpression));
    return {
      type: ExpressionType.Array,
      value,
    };
  } else if (typeof value === "object") {
    let value = Object.fromEntries(
      await Promise.all(
        Object.entries(value).map(async ([key, value]) => [
          key,
          (await addExpression(value)).toString(),
        ])
      )
    );
    return {
      type: ExpressionType.Map,
      value,
    };
  } else {
    throw new Error("Attempted to hash a value that is not an expression.");
  }
};

let addBlob = async (bytes) => {
  return new Hash(await syscall(Syscall.AddBlob, bytes));
};

let getBlob = async (hash) => {
  return await syscall(Syscall.GetBlob, hash);
};

let addExpression = async (expression) => {
  return new Hash(await syscall(Syscall.AddExpression, toInternal(expression)));
};

let getExpression = async (hash) => {
  return fromInternal(
    hash,
    await syscall(Syscall.GetExpression, hash.toString())
  );
};

let evaluate = async (hash) => {
  return new Hash(await syscall(Syscall.Evaluate, hash.toString()));
};

let template = (strings, ...placeholders) => {
  let components = [];
  for (let i = 0; i < strings.length - 1; i++) {
    let string = strings[i];
    let placeholder = placeholders[i];
    components.push(string);
    components.push(placeholder);
  }
  components.push(strings[strings.length - 1]);
  return new Template(components);
};

globalThis.console = {
  log: (...args) => syscall(Syscall.Console, "log", args),
};

globalThis.Tangram = {
  Artifact,
  Dependency,
  Directory,
  Fetch,
  File,
  Hash,
  internal: {
    ExpressionType,
    Syscall,
    System,
    syscall,
  },
  Process,
  Js,
  Symlink,
  System,
  Target,
  Template,
  addBlob,
  addExpression,
  evaluate,
  fromInternal,
  getBlob,
  getExpression,
  template,
  toInternal,
};
