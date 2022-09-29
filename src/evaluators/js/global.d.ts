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

    enum ConsoleLevel {
      Log = "log",
    }

    enum Syscall {
      Console = "console",
      AddBlob = "add_blob",
      GetBlob = "get_blob",
      AddExpression = "add_expression",
      GetExpression = "get_expression",
      Evaluate = "evaluate",
    }

    function syscall(
      syscall: Syscall.Console,
      level: ConsoleLevel,
      args: Array<any>
    );

    function syscall(syscall: Syscall.AddBlob, bytes: string): Promise<Hash>;

    function syscall(syscall: Syscall.GetBlob, hash: Hash): Promise<string>;

    function syscall(
      syscall: Syscall.AddExpression,
      expression: Expression
    ): Hash;

    function syscall(
      syscall: Syscall.GetExpression,
      hash: Hash
    ): Promise<Expression>;

    function syscall(syscall: Syscall.Evaluate, hash: Hash): Promise<Hash>;
  }

  enum System {
    Amd64Linux = "amd64_linux",
    Amd64Macos = "amd64_macos",
    Arm64Linux = "arm64_linux",
    Arm64Macos = "arm64_macos",
    Js = "js",
  }

  class Hash<T extends Expression> {
    constructor(hash: string);

    toString(): string;
  }

  type HashOr<T extends Expression> = Hash<T> | T;

  type Expression =
    | null
    | boolean
    | number
    | string
    | Artifact
    | Directory
    | File
    | Symlink
    | Dependency
    | Template
    | Fetch
    | Process
    | Target
    | Array<HashOr<Expression>>
    | { [key: string]: HashOr<Expression> };

  type OutputForExpression<T extends Expression> = T extends null
    ? null
    : T extends boolean
    ? boolean
    : T extends number
    ? number
    : T extends string
    ? string
    : T extends Artifact
    ? Artifact
    : T extends Template
    ? Template
    : T extends Js<infer U>
    ? U
    : T extends Fetch
    ? Artifact
    : T extends Process
    ? Artifact
    : T extends Target<infer U>
    ? OutputForExpression<U>
    : T extends Array<infer U>
    ? Array<OutputForExpression<U>>
    : T extends { [key: string]: infer U }
    ? { [key: string]: OutputForExpression<U> }
    : never;

  class Artifact {
    constructor(expression: HashOr<Expression>);
  }

  class Directory {
    constructor(entries: { [key: string]: HashOr<Expression> });
  }

  class File {
    constructor(blob: Hash, executable?: boolean);
  }

  class Symlink {
    constructor(target: string);
  }

  class Dependency {
    constructor(artifact: HashOr<Artifact>);
  }

  class Template {
    constructor(components: Array<HashOr<string | Artifact>>);
  }

  type JsArgs = {
    args: HashOr<Expression>;
    artifact: HashOr<Expression>;
    dependencies: { [key: string]: Hash };
    export: string;
    path: string | null;
  };

  class Js {
    constructor(args: JsArgs);
  }

  type FetchArgs = {
    hash?: string;
    unpack?: boolean;
    url: string;
  };

  class Fetch {
    constructor(args: FetchArgs);
  }

  type ProcessArgs = {
    args: HashOr<Expression>;
    command: HashOr<Artifact | Template>;
    env: HashOr<Expression>;
    system: System;
  };

  class Process {
    constructor(args: ProcessArgs);
  }

  type TargetArgs = {
    package: HashOr<Artifact>;
    name: string;
    args: HashOr<Array<HashOr<Expression>>>;
  };

  class Target {
    constructor(args: TargetArgs);
  }

  let template: (
    strings: TemplateStringsArray,
    ...placeholders: Array<Expression>
  ) => Template;

  let addBlob: (bytes: string) => Promise<Hash>;
  let addExpression: (expression: Expression) => Promise<Hash>;
  let evaluate: (hash: Hash) => Promise<Hash>;
  let getBlob: (hash: Hash) => Promise<string>;
  let getExpression: (hash: Hash) => Promise<Expression>;
}
