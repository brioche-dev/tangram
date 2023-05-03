import * as syscall from "./syscall.ts";

export class Placeholder {
	#name: string;

	static new(name: string): Placeholder {
		return new Placeholder(name);
	}

	constructor(name: string) {
		this.#name = name;
	}

	static is(value: unknown): value is Placeholder {
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

export let placeholder = Placeholder.new;
