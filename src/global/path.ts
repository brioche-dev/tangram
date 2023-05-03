import * as syscall from "./syscall.ts";
import { nullish } from "./value.ts";

export class Path {
	#components: Array<Path.Component>;

	static new(...args: Array<Path.Arg>): Path {
		// Collect the components.
		let components: Array<Path.Component> = [];
		let collectComponents = (arg: Path.Arg | nullish) => {
			if (typeof arg === "string") {
				// Push each component.
				for (let component of arg.split("/")) {
					if (component === "" || component === ".") {
						// Ignore empty and current dir components.
					} else if (component === "..") {
						components.push({ kind: "parent" });
					} else {
						components.push({
							kind: "normal",
							value: component,
						});
					}
				}
			} else if (Path.Component.is(arg)) {
				components.push(arg);
			} else if (arg instanceof Path) {
				components.push(...arg.components());
			} else if (arg instanceof Array) {
				for (let component of arg) {
					collectComponents(component);
				}
			}
		};
		for (let arg of args) {
			collectComponents(arg);
		}

		// Create the path.
		let path_ = new Path();
		for (let component of components) {
			path_.push(component);
		}

		return path_;
	}

	constructor(components: Array<Path.Component> = []) {
		this.#components = components;
	}

	static is(value: unknown): value is Path {
		return value instanceof Path;
	}

	toSyscall(): syscall.Path {
		return this.toString();
	}

	static fromSyscall(value: syscall.Path): Path {
		return path(value);
	}

	components(): Array<Path.Component> {
		return [...this.#components];
	}

	push(component: Path.Component) {
		if (component.kind === "parent") {
			let lastComponent = this.#components.at(-1);
			if (lastComponent === undefined || lastComponent.kind === "parent") {
				this.#components.push(component);
			} else {
				this.#components.pop();
			}
		} else {
			this.#components.push(component);
		}
	}

	join(other: Path.Arg): Path {
		let result = path(this);
		for (let component of path(other).components()) {
			result.push(component);
		}
		return result;
	}

	diff(src: Path.Arg): Path {
		let srcPath = path(src);
		let dstPath = path(this);

		// Remove the paths' common ancestor.
		while (true) {
			let srcComponent = srcPath.#components.at(0);
			let dstComponent = dstPath.#components.at(0);
			if (
				srcComponent &&
				dstComponent &&
				Path.Component.equal(srcComponent, dstComponent)
			) {
				srcPath.#components.shift();
				dstPath.#components.shift();
			} else {
				break;
			}
		}

		// If there is no valid path from the base to the target, then throw an error.
		if (srcPath.#components.at(0)?.kind === "parent") {
			throw new Error(
				`There is no valid path from "${srcPath}" to "${dstPath}".`,
			);
		}

		// Construct the path.
		let output = path(
			Array.from({ length: srcPath.#components.length }, () => ({
				kind: "parent",
			})),
			dstPath,
		);
		return output;
	}

	toString(): string {
		return this.#components
			.map((component) => {
				switch (component.kind) {
					case "parent": {
						return "..";
					}
					case "normal": {
						return component.value;
					}
				}
			})
			.join("/");
	}
}

export namespace Path {
	export type Component =
		| { kind: "parent" }
		| { kind: "normal"; value: string };

	export namespace Component {
		export let is = (value: unknown): value is Path.Component => {
			return (
				typeof value === "object" &&
				value !== null &&
				"kind" in value &&
				(value.kind === "parent" || value.kind === "normal")
			);
		};

		export let equal = (a: Path.Component, b: Path.Component): boolean => {
			return (
				a.kind === b.kind &&
				(a.kind === "normal" && b.kind === "normal"
					? a.value === b.value
					: true)
			);
		};
	}
}

export namespace Path {
	export type Arg = string | Path.Component | Path | Array<Arg>;

	export namespace Arg {
		export let is = (value: unknown): value is Path.Arg => {
			return (
				typeof value === "string" ||
				Path.Component.is(value) ||
				value instanceof Path ||
				(value instanceof Array && value.every(Path.Arg.is))
			);
		};
	}
}

export let path = Path.new;
