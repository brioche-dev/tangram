export let placeholder = (name: string): Placeholder => {
	return new Placeholder(name);
};

export let isPlaceholder = (value: unknown): value is Placeholder => {
	return value instanceof Placeholder;
};

export class Placeholder {
	#name: string;

	constructor(name: string) {
		this.#name = name;
	}

	async serialize(): Promise<syscall.Placeholder> {
		return {
			name: this.#name,
		};
	}

	static async deserialize(
		placeholder: syscall.Placeholder,
	): Promise<Placeholder> {
		let name = placeholder.name;
		return new Placeholder(name);
	}

	name(): string {
		return this.#name;
	}
}
