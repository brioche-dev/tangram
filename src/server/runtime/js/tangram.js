globalThis.console = {
  log: (...args) => Tangram.syscallSync("console_log", args),
};

globalThis.Tangram = {
  syscallAsync: async (opName, ...args) =>
    await Deno.core.opAsync("op_tangram_" + opName, ...args),
  syscallSync: (opName, ...args) =>
    Deno.core.opSync("op_tangram_" + opName, ...args),
};
