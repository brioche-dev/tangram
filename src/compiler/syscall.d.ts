declare function syscall(
	name: "resolve",
	specifier: string,
	referrer: string,
): string;

declare function syscall(
	name: "load",
	fileName: string,
): { version: string; text: string };

declare function syscall(name: "opened_files"): Array<string>;

declare function syscall(name: "version", fileName: string): string;
