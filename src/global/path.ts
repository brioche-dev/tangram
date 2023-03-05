export type PathLike = string | Array<PathComponent> | Path;

export type PathComponentKind = "current_dir" | "parent_dir" | "normal";

export type PathComponent =
	| { kind: "current_dir" }
	| { kind: "parent_dir" }
	| { kind: "normal"; value: string };

export let path = (path: PathLike): Path => {
	return new Path(path);
};

export let isPath = (value: unknown): value is Path => {
	return value instanceof Path;
};

export class Path {
	#components: Array<PathComponent>;

	constructor(path: PathLike) {
		if (typeof path === "string") {
			// Create the components.
			this.#components = [];

			// Split the string by the path separator.
			let components = path.split("/");

			// Add each component.
			for (let component of components) {
				if (component === "") {
					// Ignore extra separators.
				} else if (component === ".") {
					this.#components.push({ kind: "current_dir" });
				} else if (component === "..") {
					this.#components.push({ kind: "parent_dir" });
				} else {
					this.#components.push({
						kind: "normal",
						value: component,
					});
				}
			}
		} else if (Array.isArray(path)) {
			this.#components = path;
		} else {
			this.#components = path.components();
		}
	}

	components(): Array<PathComponent> {
		return [...this.#components];
	}

	parent(): Path {
		let result = path(this);
		result.push({ kind: "parent_dir" });
		return result;
	}

	push(component: PathComponent) {
		this.#components.push(component);
	}

	join(other: PathLike): Path {
		return path([...this.components(), ...path(other).components()]);
	}

	normalize(): Path {
		let components: Array<PathComponent> = [];
		for (let component of this.components()) {
			switch (component.kind) {
				case "current_dir": {
					// Skip current dir components.
					break;
				}

				case "parent_dir": {
					if (
						components.every((component) => component.kind === "parent_dir")
					) {
						// If the normalized path is zero or more parent dir components, then add a parent dir component.
						components.push(component);
					} else {
						// Otherwise, remove the last component.
						components.pop();
					}
					break;
				}

				case "normal": {
					components.push(component);
					break;
				}
			}
		}
		return path(components);
	}

	toString(): string {
		let components = [];
		for (let component of this.components()) {
			switch (component.kind) {
				case "current_dir": {
					components.push(".");
					break;
				}

				case "parent_dir": {
					components.push("..");
					break;
				}

				case "normal": {
					components.push(component.value);
					break;
				}
			}
		}
		return components.join("/");
	}
}
