/** Get the module identifiers of all documents. */
declare function syscall(name: "get_documents"): Array<string>;

/** Load the text of a module. */
declare function syscall(name: "load_module", moduleIdentifier: string): string;

/** Write to the log. */
declare function syscall(name: "log", value: string): string;

/** Resolve a module specifier from a module identifier. */
declare function syscall(
	name: "resolve_module",
	specifier: string,
	referrer: string,
): string;

/** Get the version of a module. */
declare function syscall(
	name: "get_module_version",
	moduleIdentifier: string,
): string;
