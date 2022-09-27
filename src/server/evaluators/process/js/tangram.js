globalThis.console = {
  log: (...args) => Tangram.syscall("console_log", args),
};

globalThis.Tangram = {
  syscall: (opName, ...args) => {
    return Deno.core.ops["op_tangram_" + opName](...args);
  },
  Syscall: {
    AddBlob: "add_blob",
    GetBlob: "get_blob",
    AddExpression: "add_expression",
    GetExpression: "get_expression",
    Evaluate: "evaluate",
  },
  System: {
    Amd64Linux: "amd64_linux",
    Amd64Macos: "amd64_macos",
    Arm64Linux: "arm64_linux",
    Arm64Macos: "arm64_macos",
    Js: "js",
  },
  ExpressionType: {
    Null: "null",
    Bool: "bool",
    Number: "number",
    String: "string",
    Artifact: "artifact",
    Directory: "directory",
    File: "file",
    Symlink: "symlink",
    Dependency: "dependency",
    Path: "path",
    Template: "template",
    Fetch: "fetch",
    Process: "process",
    Target: "target",
    Array: "array",
    Map: "map",
  },
};
