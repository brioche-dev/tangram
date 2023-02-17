/** Get the module identifiers of all documents. */
declare function syscall(name: "documents"): Array<string>;

/** Load the text of a module. */
declare function syscall(name: "load", moduleIdentifier: string): string;

/** Write to the log. */
declare function syscall(name: "log", value: string): string;

/** Resolve a module specifier from a module identifier. */
declare function syscall(
	name: "resolve",
	specifier: string,
	referrer: string,
): string;

/** Get the version of a module. */
declare function syscall(name: "version", moduleIdentifier: string): string;
