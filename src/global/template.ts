import { Artifact } from "./artifact.ts";
import { assert as assert_, unreachable } from "./assert.ts";
import { Relpath, Subpath } from "./path.ts";
import { Placeholder } from "./placeholder.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { MaybeNestedArray, flatten } from "./util.ts";

export let t = async (
	strings: TemplateStringsArray,
	...placeholders: Array<Unresolved<Template.Arg>>
): Promise<Template> => {
	// Collect the strings and placeholders.
	let components: Array<Unresolved<Template.Arg>> = [];
	for (let i = 0; i < strings.length - 1; i++) {
		let string = strings[i]!;
		components.push(string);
		let placeholder = placeholders[i]!;
		components.push(placeholder);
	}
	components.push(strings[strings.length - 1]!);
	return await template(...components);
};

export let template = (
	...args: Array<Unresolved<Template.Arg>>
): Promise<Template> => {
	return Template.new(...args);
};

export class Template {
	#components: Array<Template.Component>;

	static async new(
		...args: Array<Unresolved<Template.Arg>>
	): Promise<Template> {
		// Collect the components.
		let components = flatten(
			await Promise.all(
				args.map(async function map(
					arg,
				): Promise<MaybeNestedArray<Template.Component>> {
					arg = await resolve(arg);
					if (Template.Component.is(arg)) {
						return arg;
					} else if (arg instanceof Relpath || arg instanceof Subpath) {
						return arg.toString();
					} else if (arg instanceof Template) {
						return arg.components();
					} else if (arg instanceof Array) {
						return await Promise.all(arg.map(map));
					} else {
						return unreachable();
					}
				}),
			),
		).reduce<Array<Template.Component>>((components, component) => {
			components.push(component);
			return components;
		}, []);

		// Normalize the components.
		components = components.reduce<Array<Template.Component>>(
			(components, component) => {
				let lastComponent = components.at(-1);
				if (component === "") {
					// Ignore empty string components.
				} else if (
					typeof lastComponent === "string" &&
					typeof component === "string"
				) {
					// Merge adjacent string components.
					components.splice(-1, 1, lastComponent + component);
				} else {
					components.push(component);
				}
				return components;
			},
			[],
		);

		return new Template(components);
	}

	constructor(components: Array<Template.Component>) {
		this.#components = components;
	}

	static is(value: unknown): value is Template {
		return value instanceof Template;
	}

	static expect(value: unknown): Template {
		assert_(Template.is(value));
		return value;
	}

	static assert(value: unknown): asserts value is Template {
		assert_(Template.is(value));
	}

	/** Join an array of templates with a separator. */
	static async join(
		separator: Unresolved<Template.Arg>,
		...args: Array<Unresolved<Template.Arg>>
	): Promise<Template> {
		let separatorTemplate = await template(separator);
		let argTemplates = await Promise.all(args.map((arg) => template(arg)));
		argTemplates = argTemplates.filter((arg) => arg.components().length > 0);
		let templates = [];
		for (let i = 0; i < argTemplates.length; i++) {
			if (i > 0) {
				templates.push(separatorTemplate);
			}
			let argTemplate = argTemplates[i];
			assert_(argTemplate);
			templates.push(argTemplate);
		}
		return template(...templates);
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
		export let is = (value: unknown): value is Component => {
			return (
				typeof value === "string" ||
				Artifact.is(value) ||
				value instanceof Placeholder
			);
		};

		export let toSyscall = (
			component: Component,
		): syscall.Template.Component => {
			if (typeof component === "string") {
				return {
					kind: "string",
					value: component,
				};
			} else if (Artifact.is(component)) {
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
			component: syscall.Template.Component,
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

export namespace Template {
	export type Arg =
		| undefined
		| Component
		| Relpath
		| Subpath
		| Template
		| Array<Arg>;
}
