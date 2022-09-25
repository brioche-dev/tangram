globalThis.console = {
  log: (...args) => Tangram.syscall("console_log", args),
};

globalThis.Tangram = {
  syscall: (opName, ...args) => Deno.core.ops["op_tangram_" + opName](...args),
};
