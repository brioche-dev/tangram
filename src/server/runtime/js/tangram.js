globalThis.console = {
  log: (...args) => Deno.core.opSync("op_tangram_console_log", args),
};

globalThis.Tangram = {
  evaluate: async (value) => {
    return await Deno.core.opAsync("op_tangram_evaluate", value);
  },

  fetch: (args) => {
    return Deno.core.opSync("op_tangram_fetch", args);
  },

  System: {
    Amd64Linux: "amd64_linux",
    Amd64Macos: "amd64_macos",
    Arm64Linux: "arm64_linux",
    Arm64Macos: "arm64_macos",
  },

  template: (strings, ...placeholders) => {
    return Deno.core.opSync("op_tangram_template", { placeholders, strings });
  },
};
