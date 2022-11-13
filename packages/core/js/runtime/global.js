let bs = globalThis.__bootstrap;
Object.defineProperties(globalThis, {
	TextDecoder: { value: bs.encoding.TextDecoder },
	TextEncoder: { value: bs.encoding.TextEncoder },
	URL: { value: bs.url.URL },
	URLPattern: { value: bs.urlPattern.URLPattern },
	URLSearchParams: { value: bs.url.URLSearchParams },
});

let stringify = (value) => {
	if (value === undefined) {
		return "undefined";
	} else if (value === null) {
		return "null";
	} else if (Array.isArray(value)) {
		return `[${value.map(stringify).join(", ")}]`;
	} else if (value instanceof Error) {
		return value.stack;
	} else if (value instanceof Promise) {
		return "Promise";
	} else if (typeof value === "object") {
		let constructorName = "";
		if (
			value.constructor?.name !== undefined &&
			value.constructor.name !== "Object"
		) {
			constructorName = `${value.constructor.name} `;
		}
		let entries = Object.entries(value).map(
			([key, value]) => `${key}: ${stringify(value)}`,
		);
		return `${constructorName}{ ${entries.join(", ")} }`;
	} else if (typeof value === "function") {
		return `[Function: ${value.name || "(anonymous)"}]`;
	} else {
		return JSON.stringify(value);
	}
};

Object.defineProperties(globalThis, {
	stringify: { value: stringify },
});

let console = {
	log: (...args) => {
		let string = args.map((arg) => stringify(arg)).join(" ");
		syscall(Syscall.Print, string);
	},
	error: (...args) => {
		let string = args.map((arg) => stringify(arg)).join(" ");
		syscall(Syscall.Print, string);
	},
};

Object.defineProperties(globalThis, {
	console: { value: console },
});

function syscall(syscall, ...args) {
	let opName = "op_tg_" + syscall;
	switch (syscall) {
		case "print":
			return Deno.core.opSync(opName, ...args);
		case "deserialize":
			return Deno.core.opSync(opName, ...args);
		case "add_blob":
			return Deno.core.opAsync(opName, ...args);
		case "get_blob":
			return Deno.core.opAsync(opName, ...args);
		case "add_expression":
			return Deno.core.opAsync(opName, ...args);
		case "get_expression":
			return Deno.core.opAsync(opName, ...args);
		case "evaluate":
			return Deno.core.opAsync(opName, ...args);
	}
}

Object.defineProperties(globalThis, {
	syscall: { value: syscall },
});
