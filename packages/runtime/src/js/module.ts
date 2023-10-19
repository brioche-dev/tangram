import { assert } from "./assert.ts";
import { encoding } from "./syscall.ts";
import { URL } from "whatwg-url";

export type Module =
	| { kind: "document"; value: Document }
	| { kind: "library"; value: Library }
	| { kind: "normal"; value: Normal };

export type Document = {
	packagePath: string;
	path: string;
};

export type Library = {
	path: string;
};

export type Normal = {
	packageId: string;
	path: string;
};

export namespace Module {
	export let toUrl = (module: Module): string => {
		let data = encoding.hex.encode(
			encoding.utf8.encode(encoding.json.encode(module)),
		);
		return `tangram://${data}/${module.value.path}`;
	};

	export let fromUrl = (string: string): Module => {
		let url = new URL(string);
		return encoding.json.decode(
			encoding.utf8.decode(encoding.hex.decode(url.host)),
		) as Module;
	};
}
