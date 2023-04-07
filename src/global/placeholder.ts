import * as syscall from "./syscall.ts";

export let placeholder = (name: string): Placeholder => {
	return new Placeholder(name);
};

export class Placeholder {
	#name: string;

	constructor(name: string) {
		this.#name = name;
	}

	static isPlaceholder(value: unknown): value is Placeholder {
		return value instanceof Placeholder;
	}

	toSyscall(): syscall.Placeholder {
		return {
			name: this.#name,
		};
	}

	static fromSyscall(placeholder: syscall.Placeholder): Placeholder {
		let name = placeholder.name;
		return new Placeholder(name);
	}

	name(): string {
		return this.#name;
	}
}
