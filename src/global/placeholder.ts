export let placeholder = (name: string) => {
	return Placeholder.new(name);
};

export class Placeholder {
	#name: string;

	constructor(name: string) {
		this.#name = name;
	}

	static new(name: string): Placeholder {
		return new Placeholder(name);
	}

	static is(value: unknown): value is Placeholder {
		return value instanceof Placeholder;
	}

	name(): string {
		return this.#name;
	}
}
