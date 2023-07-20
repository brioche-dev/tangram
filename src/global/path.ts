import { assert as assert_, unreachable } from "./assert.ts";
import * as syscall from "./syscall.ts";

export let relpath = (...args: Array<Relpath.Arg>): Relpath => {
	return Relpath.new(...args);
};

export let subpath = (...args: Array<Subpath.Arg>): Subpath => {
	return Subpath.new(...args);
};

type RelpathConstructorArg = {
	parents?: number;
	subpath?: Subpath;
};

export class Relpath {
	#parents: number;
	#subpath: Subpath;

	static new(...args: Array<Relpath.Arg>): Relpath {
		return args.reduce(function reduce(path: Relpath, arg: Relpath.Arg) {
			if (typeof arg === "string") {
				for (let component of arg.split("/")) {
					if (component === "" || component === ".") {
						continue;
					} else if (component === "..") {
						path = path.parent();
					} else {
						path.#subpath.push(component);
					}
				}
			} else if (arg instanceof Relpath) {
				for (let i = 0; i < arg.#parents; i++) {
					path.parent();
				}
				path.#subpath.join(arg.#subpath);
			} else if (arg instanceof Subpath) {
				path.#subpath.join(arg);
			} else if (arg instanceof Array) {
				arg.forEach((arg) => reduce(path, arg));
			} else {
				return unreachable();
			}
			return path;
		}, new Relpath());
	}

	constructor(arg?: RelpathConstructorArg) {
		this.#parents = arg?.parents ?? 0;
		this.#subpath = arg?.subpath ?? new Subpath();
	}

	static is(value: unknown): value is Relpath {
		return value instanceof Relpath;
	}

	toSyscall(): syscall.Relpath {
		return this.toString();
	}

	static fromSyscall(value: syscall.Relpath): Relpath {
		return Relpath.new(value);
	}

	isEmpty(): boolean {
		return this.#parents == 0 && this.#subpath.isEmpty();
	}

	parents(): number {
		return this.#parents;
	}

	subpath(): Subpath {
		return this.#subpath;
	}

	parent(): Relpath {
		if (this.#subpath.isEmpty()) {
			this.#parents += 1;
		} else {
			this.#subpath.pop();
		}
		return this;
	}

	join(other: Relpath.Arg): Relpath {
		other = Relpath.new(other);
		for (let i = 0; i < other.#parents; i++) {
			this.parent();
		}
		this.#subpath.join(other.#subpath);
		return this;
	}

	extension(): string | undefined {
		return this.#subpath.extension();
	}

	toSubpath(): Subpath {
		if (this.#parents > 0) {
			throw new Error("Cannot convert to subpath.");
		}
		return this.#subpath;
	}

	toString(): string {
		let string = "";
		for (let i = 0; i < this.#parents; i++) {
			string += "../";
		}
		string += this.#subpath.toString();
		return string;
	}
}

export namespace Relpath {
	export type Arg = undefined | string | Subpath | Relpath | Array<Arg>;

	export namespace Arg {
		export let is = (value: unknown): value is Relpath.Arg => {
			return (
				value === undefined ||
				typeof value === "string" ||
				value instanceof Subpath ||
				value instanceof Relpath ||
				(value instanceof Array && value.every(Relpath.Arg.is))
			);
		};

		export let expect = (value: unknown): Relpath.Arg => {
			assert_(is(value));
			return value;
		};

		export let assert = (value: unknown): asserts value is Relpath.Arg => {
			assert_(is(value));
		};
	}
}

export class Subpath {
	#components: Array<string>;

	static new(...args: Array<Subpath.Arg>): Subpath {
		return Relpath.new(...args).toSubpath();
	}

	constructor(components?: Array<string>) {
		this.#components = components ?? [];
	}

	static is(value: unknown): value is Subpath {
		return value instanceof Subpath;
	}

	toSyscall(): syscall.Subpath {
		return this.toString();
	}

	static fromSyscall(value: syscall.Subpath): Subpath {
		return subpath(value);
	}

	components(): Array<string> {
		return [...this.#components];
	}

	isEmpty(): boolean {
		return this.#components.length == 0;
	}

	join(other: Subpath): Subpath {
		this.#components.push(...other.#components);
		return this;
	}

	push(component: string) {
		this.#components.push(component);
	}

	pop() {
		this.#components.pop();
	}

	extension(): string | undefined {
		return this.#components.at(-1)?.split(".").at(-1);
	}

	toRelpath(): Relpath {
		return Relpath.new(this);
	}

	toString(): string {
		return this.#components.join("/");
	}
}

export namespace Subpath {
	export type Arg = undefined | string | Subpath | Array<Arg>;
}
