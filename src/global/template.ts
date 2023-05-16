import { Artifact } from "./artifact.ts";
import { assert, unreachable } from "./assert.ts";
import { Path } from "./path.ts";
import { Placeholder } from "./placeholder.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";

export let t = async (
	strings: TemplateStringsArray,
	...placeholders: Array<Unresolved<Template.Arg>>
): Promise<Template> => {
	// Collect the strings and placeholders.
	let components: Array<Unresolved<Template.Arg>> = [];
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

export class Template {
	#components: Array<Template.Component>;

	static async new(
		...args: Array<Unresolved<Template.Arg>>
	): Promise<Template> {
		// Collect the components.
		let components: Array<Template.Component> = [];
		let collectComponents = (arg: Template.Arg) => {
			if (Template.Component.is(arg)) {
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

		// Remove leading indentation.
		components = stripLeadingWhitespace(components);

		return new Template(components);
	}

	constructor(components: Array<Template.Component>) {
		this.#components = components;
	}

	static is(value: unknown): value is Template {
		return value instanceof Template;
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
			assert(argTemplate);
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
		): syscall.TemplateComponent => {
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

export namespace Template {
	export type Arg = undefined | Component | Path | Template | Array<Arg>;

	export namespace Arg {
		export let is = (value: unknown): value is Template.Arg => {
			return (
				Template.Component.is(value) ||
				value instanceof Path ||
				value instanceof Template ||
				(value instanceof Array && value.every(Template.Arg.is))
			);
		};
	}
}

export let template = Template.new;

// Compute the minimum indentation level of a string. Returns undefined if the string is on one line.
let minIndentLevel = (s: string): string | undefined => {
	let lines: Array<string> = s.split("\n");

	if (lines.length == 1) {
		return undefined;
	}

	// Strip lines with only whitespace.
	lines = lines.filter((line) => {
		let matches = /^\s*$/.exec(line);
		return !matches;
	});

	// Find lines with zero or more leading whitespaces and find the leading whitespace of minimum length.
	lines = lines
		.map((line) => {
			let matches = /^\s*/.exec(line);
			return matches?.map((s) => s) ?? [];
		})
		.flat();

	if (lines.length == 0) {
		return undefined;
	} else {
		return lines.reduce((acc, str) => {
			let l1 = acc?.length ?? 0;
			let l2 = str?.length ?? 0;
			return l1 < l2 ? acc : str;
		});
	}
};

// Remove the leading whitespace from string components in a template, including empty lines and any leading indentation.
// Note: does not handle mixed tab/spaces for indentation.
let stripLeadingWhitespace = (
	components: Array<Template.Component>,
): Array<Template.Component> => {
	let minIndent: string | undefined = undefined;

	// Compute the minimum indentation level.
	for (let component of components) {
		if (typeof component === "string") {
			let indent = minIndentLevel(component);
			if (indent && !minIndent) {
				minIndent = indent;
			} else if (indent && minIndent && indent.length < minIndent.length) {
				minIndent = indent;
			}
		}
	}

	// If there was some indentation, replace all occurrences of it.
	if (minIndent) {
		let indent = minIndent; // Needed for type narrowing.
		components = components.map((component) => {
			if (typeof component === "string") {
				return component
					.split("\n")
					.map((line) => {
						if (line.startsWith(indent)) {
							line = line.replace(indent, "");
						}
						return line;
					})
					.join("\n");
			} else {
				return component;
			}
		});
	}

	return components;
};
