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
      path: string | null;
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
  }

  class Hash<T extends Expression = Expression> {
    constructor(hash: string);

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
    | Template
    | Js<Output>
    | Fetch
    | Process
    | Target<Output>
    | Array<Expression<Output>>
    | { [key: string]: Expression<Output> };

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
    : T extends Js<infer O>
    ? OutputForExpression<O>
    : T extends Fetch
    ? Artifact
    : T extends Process
    ? Artifact
    : T extends Target<infer O>
    ? OutputForExpression<O>
    : T extends Array<infer V extends Expression>
    ? Array<OutputForExpression<V>>
    : T extends { [key: string]: infer V extends Expression }
    ? { [key: string]: OutputForExpression<V> }
    : never;

  class Artifact {
    constructor(expression: Expression);
  }

  class Directory {
    constructor(entries: { [key: string]: Expression });
  }

  class File {
    constructor(blob: Expression<string>, executable?: boolean);
  }

  class Symlink {
    constructor(target: string);
  }

  class Dependency {
    constructor(artifact: Artifact, path?: string | null);
  }

  class Template {
    constructor(components: Array<string | Artifact | Template>);
  }

  type JsArgs = {
    args: Expression<Array<Expression<string | Artifact | Template>>>;
    artifact: Expression<Artifact>;
    dependencies: { [key: string]: Expression<Artifact> };
    export: string;
    path: string | null;
  };

  class Js<O extends Expression> {
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
    args: Array<Expression>;
    command: Expression<string | Artifact | Template>;
    env: Expression<{
      [key: string]: Expression<string | Artifact | Template>;
    }>;
    system: System;
  };

  class Process {
    constructor(args: ProcessArgs);
  }

  type TargetArgs = {
    package: Expression<Artifact>;
    name: string;
    args: Array<Expression>;
  };

  class Target<O extends Expression> {
    constructor(args: TargetArgs);
  }

  let template: (
    strings: TemplateStringsArray,
    ...placeholders: Array<Expression<string | Artifact | Template>>
  ) => Template;

  let addBlob: (bytes: string) => Promise<Hash>;

  let addExpression: <T extends Expression>(
    expression: Expression<T>
  ) => Promise<Hash<T>>;

  let evaluate: <O extends Expression>(
    hash: Hash<Expression<O>>
  ) => Promise<Hash<O>>;

  let getBlob: (hash: Hash) => Promise<string>;

  let getExpression: (hash: Hash) => Promise<Expression>;
}
