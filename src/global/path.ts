export type PathLike = string | Array<PathComponent> | Path;

export type PathComponentKind = "parent_dir" | "normal";

export type PathComponent =
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

	constructor(pathLike: PathLike) {
		if (typeof pathLike === "string") {
			// Create the components.
			this.#components = [];

			if (pathLike.startsWith("/")) {
				throw new Error("Absolute paths are not allowed.");
			}

			// Split the string by the path separator.
			let components = pathLike.split("/");

			// Push each component.
			for (let component of components) {
				if (component === "") {
					throw new Error("Empty path components are not allowed.");
				} else if (component === ".") {
					// Ignore current dir components.
				} else if (component === "..") {
					this.#components.push({ kind: "parent_dir" });
				} else {
					this.#components.push({
						kind: "normal",
						value: component,
					});
				}
			}
		} else if (pathLike instanceof Array) {
			this.#components = pathLike;
		} else {
			this.#components = pathLike.components();
		}
	}

	components(): Array<PathComponent> {
		return [...this.#components];
	}

	push(component: PathComponent) {
		if (component.kind === "parent_dir") {
			let lastComponent = this.#components.at(-1);
			if (lastComponent === undefined || lastComponent.kind === "parent_dir") {
				this.#components.push(component);
			} else {
				this.#components.pop();
			}
		} else {
			this.#components.push(component);
		}
	}

	parent(): Path {
		let result = path(this);
		result.push({ kind: "parent_dir" });
		return result;
	}

	join(other: PathLike): Path {
		let result = path(this);
		for (let component of path(other).components()) {
			result.push(component);
		}
		return result;
	}

	toString(): string {
		let string = this.#components
			.map((component) => {
				switch (component.kind) {
					case "parent_dir": {
						return "..";
					}
					case "normal": {
						return component.value;
					}
				}
			})
			.join("/");

		let firstComponent = this.#components[0];
		if (firstComponent === undefined) {
			return ".";
		} else if (firstComponent.kind === "parent_dir") {
			return string;
		} else {
			return `./${string}`;
		}
	}
}
