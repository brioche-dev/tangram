import { Artifact } from "./artifact.ts";
import { assert, unreachable } from "./assert.ts";
import { Path } from "./path.ts";
import { Placeholder } from "./placeholder.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { nullish } from "./value.ts";

export namespace Template {
	export type Arg = Component | Path | Template | Array<Arg>;
}

export let t = async (
	strings: TemplateStringsArray,
	...placeholders: Array<Unresolved<Template.Arg | nullish>>
): Promise<Template> => {
	// Collect the strings and placeholders.
	let components: Array<Unresolved<Template.Arg | nullish>> = [];
	for (let i = 0; i < strings.length - 1; i++) {
		// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
		let string = strings[i]!;
		components.push(string);
		// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
		let placeholder = placeholders[i]!;
		components.push(placeholder);
	}
	// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
	components.push(strings[strings.length - 1]!);
	return await template(...components);
};

export let template = async (
	...args: Array<Unresolved<Template.Arg | nullish>>
): Promise<Template> => {
	// Collect the components.
	let components: Array<Template.Component> = [];
	let collectComponents = (arg: Template.Arg | nullish) => {
		if (Template.Component.isTemplateComponent(arg)) {
			components.push(arg);
		} else if (arg instanceof Path) {
			components.push(arg.toString());
		} else if (arg instanceof Template) {
			components.push(...arg.components());
		} else if (arg instanceof Array) {
			for (let component of arg) {
				collectComponents(component);
			}
		}
	};
	for (let arg of await Promise.all(args.map(resolve))) {
		collectComponents(arg);
	}

	// Normalize the components.
	let normalizedComponents: Array<Template.Component> = [];
	for (let component of components) {
		let lastComponent = normalizedComponents.at(-1);
		if (component === "") {
			// Ignore empty string components.
			continue;
		} else if (
			typeof lastComponent === "string" &&
			typeof component === "string"
		) {
			// Merge adjacent string components.
			normalizedComponents.splice(-1, 1, lastComponent + component);
		} else {
			normalizedComponents.push(component);
		}
	}
	components = normalizedComponents;

	return new Template(components);
};

export class Template {
	#components: Array<Template.Component>;

	constructor(components: Array<Template.Component>) {
		this.#components = components;
	}

	static isTemplate(value: unknown): value is Template {
		return value instanceof Template;
	}

	/** Join an array of templates with a separator. */
	static async join(
		separator: Unresolved<Template.Arg>,
		...args: Array<Unresolved<Template.Arg | nullish>>
	): Promise<Template> {
		let resolvedSeparator = await template(separator);
		let resolvedArgs = await Promise.all(args.map((arg) => template(arg)));
		let components = [];
		for (let i = 0; i < resolvedArgs.length; i++) {
			if (i > 0) {
				components.push(resolvedSeparator);
			}
			let arg = resolvedArgs[i];
			assert(arg);
			components.push(arg);
		}
		return template(...components);
	}

	toSyscall(): syscall.Template {
		let components = this.#components.map((component) =>
			Template.Component.toSyscall(component),
		);
		return {
			components,
		};
	}

	static fromSyscall(value: syscall.Template): Template {
		let components = value.components.map((component) =>
			Template.Component.fromSyscall(component),
		);
		return new Template(components);
	}

	components(): Array<Template.Component> {
		return [...this.#components];
	}
}

export namespace Template {
	export type Component = string | Artifact | Placeholder;

	export namespace Component {
		export let isTemplateComponent = (value: unknown): value is Component => {
			return (
				typeof value === "string" ||
				Artifact.isArtifact(value) ||
				value instanceof Placeholder
			);
		};

		export let toSyscall = (
			component: Component,
		): syscall.TemplateComponent => {
			if (typeof component === "string") {
				return {
					kind: "string",
					value: component,
				};
			} else if (Artifact.isArtifact(component)) {
				return {
					kind: "artifact",
					value: Artifact.toSyscall(component),
				};
			} else if (component instanceof Placeholder) {
				return {
					kind: "placeholder",
					value: component.toSyscall(),
				};
			} else {
				return unreachable();
			}
		};

		export let fromSyscall = (
			component: syscall.TemplateComponent,
		): Component => {
			switch (component.kind) {
				case "string": {
					return component.value;
				}
				case "artifact": {
					return Artifact.fromSyscall(component.value);
				}
				case "placeholder": {
					return Placeholder.fromSyscall(component.value);
				}
				default: {
					return unreachable();
				}
			}
		};
	}
}
