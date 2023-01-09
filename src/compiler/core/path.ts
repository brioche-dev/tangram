export type PathLike = string | Array<PathComponent> | Path;

export let path = (path: PathLike): Path => {
	return new Path(path);
};

export type PathComponentType =
	| "root_dir"
	| "current_dir"
	| "parent_dir"
	| "normal";

export type PathComponent =
	| { type: "root_dir" }
	| { type: "current_dir" }
	| { type: "parent_dir" }
	| { type: "normal"; value: string };

export class Path {
	#components: Array<PathComponent>;

	constructor(path: PathLike) {
		if (typeof path === "string") {
			// Handle a string.
			this.#components = [];

			// If the string is empty, we are done.
			if (path.length === 0) {
				return this;
			}

			// Split the string by path separator.
			let components = path.split("/");

			// If the first component is the empty string, add a root dir component.
			if (components.at(0) === "") {
				this.#components.push({ type: "root_dir" });
				components.shift();
			}

			// Add each component.
			for (let component of components) {
				if (component === "") {
					// Ignore extra separators.
				} else if (component === ".") {
					this.#components.push({ type: "current_dir" });
				} else if (component === "..") {
					this.#components.push({ type: "parent_dir" });
				} else {
					this.#components.push({
						type: "normal",
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

	parent(): Path | undefined {
		// Get the path's components.
		let components = this.components();

		// If the path is empty or is just a root component, return undefined.
		if (
			components.length === 0 ||
			(components.length === 1 && components.at(0)?.type === "root_dir")
		) {
			return undefined;
		}

		// Return a new path with the last component omitted.
		components.pop();
		return path(components);
	}

	join(other: PathLike): Path {
		let components = this.components();
		for (let component of path(other).components()) {
			switch (component.type) {
				case "root_dir": {
					// Replace all components with a single root dir component.
					components = [component];
					break;
				}
				default: {
					components.push(component);
					break;
				}
			}
		}
		return path(components);
	}

	normalize(): Path {
		let components: Array<PathComponent> = [];
		for (let component of this.components()) {
			switch (component.type) {
				case "root_dir": {
					// Replace all components with a single root dir component.
					components = [component];
					break;
				}
				case "current_dir": {
					// Skip current dir components.
					break;
				}
				case "parent_dir": {
					if (
						components.length === 1 &&
						components.at(0)?.type === "root_dir"
					) {
						// If the normalized path has one component which is a root dir component, then do nothing.
					} else if (
						components.every((component) => component.type === "parent_dir")
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
		let string = "";
		for (let component of this.components()) {
			switch (component.type) {
				case "root_dir": {
					string += "/";
					break;
				}
				case "current_dir": {
					string += "./";
					break;
				}
				case "parent_dir": {
					string += "../";
					break;
				}
				case "normal": {
					string += component.value + "/";
					break;
				}
			}
		}
		if (string.length > 1 && string.endsWith("/")) {
			string = string.slice(0, -1);
		}
		return string;
	}
}
