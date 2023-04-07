import * as syscall from "./syscall.ts";

export namespace base64 {
	export let decode = (value: string): Uint8Array => {
		return syscall.base64.decode(value);
	};

	export let encode = (value: Uint8Array): string => {
		return syscall.base64.encode(value);
	};
}

export namespace hex {
	export let decode = (value: string): Uint8Array => {
		return syscall.hex.decode(value);
	};

	export let encode = (value: Uint8Array): string => {
		return syscall.hex.encode(value);
	};
}

export namespace json {
	export let decode = (value: string): unknown => {
		return syscall.json.decode(value);
	};

	export let encode = (value: any): string => {
		return syscall.json.encode(value);
	};
}

export namespace toml {
	export let decode = (value: string): unknown => {
		return syscall.toml.decode(value);
	};

	export let encode = (value: any): string => {
		return syscall.toml.encode(value);
	};
}

export namespace utf8 {
	export let decode = (value: Uint8Array): string => {
		return syscall.utf8.decode(value);
	};

	export let encode = (value: string): Uint8Array => {
		return syscall.utf8.encode(value);
	};
}

export namespace yaml {
	export let decode = (value: string): unknown => {
		return syscall.yaml.decode(value);
	};

	export let encode = (value: any): string => {
		return syscall.yaml.encode(value);
	};
}
