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
  Print: "print",
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
  let opName = "op_tangram_" + syscall;
  switch (syscall) {
    case Syscall.Print:
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

  static fromJson(expression) {
    let root = new Hash(expression.value.root);
    return new Artifact(root);
  }

  async toJson() {
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

  static fromJson(expression) {
    let entries = Object.fromEntries(
      Object.entries(expression.value.entries).map(([key, value]) => [
        key,
        new Hash(value),
      ])
    );
    return new Directory(entries);
  }

  async toJson() {
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

  static fromJson(expression) {
    let blob = new Hash(expression.value.blob);
    let executable = expression.value.executable;
    return new File(blob, executable);
  }

  async toJson() {
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

  static fromJson(expression) {
    return new Symlink(expression.value.target);
  }

  async toJson() {
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
  path;

  constructor(artifact, path) {
    this.artifact = artifact;
    this.path = path ?? null;
  }

  static fromJson(expression) {
    return new Dependency(
      new Hash(expression.value.artifact),
      expression.value.path
    );
  }

  async toJson() {
    let artifact = await addExpression(this.artifact);
    return {
      type: ExpressionType.Dependency,
      value: {
        artifact: artifact.toString(),
        path: this.path,
      },
    };
  }
}

class Template {
  components;

  constructor(components) {
    this.components = components;
  }

  static fromJson(expression) {
    return new Template(
      expression.value.components.map((string) => new Hash(string))
    );
  }

  async toJson() {
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

  static fromJson(expression) {
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

  async toJson() {
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
        args: args.toString(),
        artifact: artifact.toString(),
        dependencies,
        name: this.name,
        path: this.path,
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

  static fromJson(expression) {
    return new Fetch({
      url: expression.value.url,
      hash: expression.value.hash,
      unpack: expression.value.unpack,
    });
  }

  async toJson() {
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

  constructor(args) {
    this.system = args.system;
    this.env = args.env;
    this.command = args.command;
    this.args = args.args;
    this.hash = args.hash;
  }

  static fromJson(expression) {
    let system = expression.value.system;
    let env = new Hash(expression.value.env);
    let command = new Hash(expression.value.command);
    let args = new Hash(expression.value.args);
    let hash = expression.value.hash;
    return new Process({ system, env, command, args, hash });
  }

  async toJson() {
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
        hash: this.hash,
      },
    };
  }
}

class Target {
  package;
  name;
  args;

  constructor(args) {
    this.package = args.package;
    this.name = args.name;
    this.args = args.args;
  }

  static fromJson(expression) {
    return new Target({
      package: new Hash(expression.value.package),
      name: expression.value.name,
      args: new Hash(expression.value.args),
    });
  }

  async toJson() {
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
}

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

let fromJson = async (expression) => {
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
      return Artifact.fromJson(expression);
    case ExpressionType.Directory:
      return Directory.fromJson(expression);
    case ExpressionType.File:
      return File.fromJson(expression);
    case ExpressionType.Symlink:
      return Symlink.fromJson(expression);
    case ExpressionType.Dependency:
      return Dependency.fromJson(expression);
    case ExpressionType.Template:
      return Template.fromJson(expression);
    case ExpressionType.Js:
      return Js.fromJson(expression);
    case ExpressionType.Fetch:
      return Fetch.fromJson(expression);
    case ExpressionType.Process:
      return Process.fromJson(expression);
    case ExpressionType.Target:
      return Target.fromJson(expression);
    case ExpressionType.Array:
      return await Promise.all(
        expression.value.map(
          async (value) => await getExpression(new Hash(value))
        )
      );
    case ExpressionType.Map:
      return Object.fromEntries(
        await Promise.all(
          Object.entries(expression.value).map(async ([key, value]) => [
            key,
            await getExpression(new Hash(value)),
          ])
        )
      );
    default:
      throw new Error(`Invalid expression type "${expression.type}".`);
  }
};

let toJson = async (expression) => {
  if (expression === null) {
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
      })
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
  if (expression instanceof Hash) {
    return expression;
  }
  return new Hash(
    await syscall(Syscall.AddExpression, await toJson(expression))
  );
};

let getExpression = async (hash) => {
  return fromJson(await syscall(Syscall.GetExpression, hash.toString()));
};

let evaluate = async (hash) => {
  return new Hash(await syscall(Syscall.Evaluate, hash.toString()));
};

globalThis.console = {
  log: (...args) => {
    let string = args.map(JSON.stringify).join(" ");
    syscall(Syscall.Print, string);
  },
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
  fromJson,
  getBlob,
  getExpression,
  template,
  toJson,
};
