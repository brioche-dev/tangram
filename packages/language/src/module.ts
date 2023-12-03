import { assert } from "./assert.ts";
import { encoding } from "./syscall.ts";

export type Module =
	| { kind: "document"; value: Document }
	| { kind: "library"; value: Library }
	| { kind: "normal"; value: Normal };

export type Document = {
	package: string;
	path: string;
};

export type Library = {
	path: string;
};

export type Normal = {
	lock: string;
	package: string;
	path: string;
};

export namespace Module {
	export let toUrl = (module: Module): string => {
		let data = encoding.hex.encode(
			encoding.utf8.encode(encoding.json.encode(module)),
		);
		return `tangram://${data}/${module.value.path}`;
	};

	export let fromUrl = (url: string): Module => {
		let match = url.match(/^tangram:\/\/([0-9a-f]+)/);
		assert(match);
		let [_, data] = match;
		assert(data !== undefined);
		return encoding.json.decode(
			encoding.utf8.decode(encoding.hex.decode(data)),
		) as Module;
	};
}
