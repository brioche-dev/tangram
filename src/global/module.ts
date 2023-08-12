import { Block } from "./block";

type ConstructorArg = {
	package: Block;
	path: string;
};

export class Module {
	#package: Block;
	#path: string;

	constructor(arg: ConstructorArg) {
		this.#package = arg.package;
		this.#path = arg.path;
	}

	package(): Block {
		return this.#package;
	}

	path(): string {
		return this.#path;
	}
}
