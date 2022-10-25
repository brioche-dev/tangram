type URL = any;

type TextEncoder = any;

type TextDecoder = any;

declare module console {
	function log(...args: any[]): void;
}

interface ImportMeta {
	url: string;
}
