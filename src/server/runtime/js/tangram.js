globalThis.console = {
  log: (...args) => Deno.core.opSync("op_tangram_console_log", args),
};

globalThis.Tangram = {
  artifact: (object_hash) => {
    return Deno.core.opSync("op_tangram_artifact", object_hash);
  },

  dependency: async (artifact) => {
    return await Deno.core.opAsync("op_tangram_dependency", artifact);
  },

  directory: async (entries) => {
    entries = Object.fromEntries(
      await Promise.all(
        Object.entries(entries).map(([key, value]) =>
          Promise.resolve(value).then((value) => [key, value])
        )
      )
    );
    return await Deno.core.opAsync("op_tangram_directory", entries);
  },

  evaluate: async (expression) => {
    return await Deno.core.opAsync("op_tangram_evaluate", expression);
  },

  fetch: (args) => {
    return Deno.core.opSync("op_tangram_fetch", args);
  },

  file: async (blob, options) => {
    return await Deno.core.opAsync("op_tangram_file", blob, options);
  },

  path: (artifact, path) => {
    return Deno.core.opSync("op_tangram_path", artifact, path);
  },

  process: (args) => {
    return Deno.core.opSync("op_tangram_process", args);
  },

  symlink: async (target) => {
    return await Deno.core.opAsync("op_tangram_symlink", target);
  },

  System: {
    Amd64Linux: "amd64_linux",
    Amd64Macos: "amd64_macos",
    Arm64Linux: "arm64_linux",
    Arm64Macos: "arm64_macos",
  },

  target: (args) => {
    return Deno.core.opSync("op_tangram_target", args);
  },

  template: (strings, ...placeholders) => {
    return Deno.core.opSync("op_tangram_template", { placeholders, strings });
  },
};
