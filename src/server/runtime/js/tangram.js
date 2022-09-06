globalThis.console = {
  log: (...args) => Deno.core.opSync("op_tangram_console_log", args),
};

globalThis.Tangram = {
  artifact: (objectHash) => {
    return Deno.core.opSync("op_tangram_artifact", objectHash);
  },

  evaluate: async (value) => {
    return await Deno.core.opAsync("op_tangram_evaluate", value);
  },

  fetch: (args) => {
    return Deno.core.opSync("op_tangram_fetch", args);
  },

  path: (artifact, path) => {
    return Deno.core.opSync("op_tangram_path", artifact, path);
  },

  process: (args) => {
    return Deno.core.opSync("op_tangram_process", args);
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
