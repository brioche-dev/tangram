import { Args } from "./args.ts";
import { Artifact } from "./artifact.ts";
import { assert as assert_, unreachable } from "./assert.ts";
import { Unresolved } from "./resolve.ts";

export let t = async (
	strings: TemplateStringsArray,
	...placeholders: Args<Template.Arg>
): Promise<Template> => {
	// Collect the strings and placeholders.
	let components: Args<Template.Arg> = [];
	for (let i = 0; i < strings.length - 1; i++) {
		let string = strings[i]!;
		components.push(string);
		let placeholder = placeholders[i]!;
		components.push(placeholder);
	}
	components.push(strings[strings.length - 1]!);
	return await template(...components);
};

export let template = (...args: Args<Template.Arg>): Promise<Template> => {
	return Template.new(...args);
};

export class Template {
	#components: Array<Template.Component>;

	constructor(components: Array<Template.Component>) {
		this.#components = components;
	}

	static async new(...args: Args<Template.Arg>): Promise<Template> {
		type Apply = {
			components: Array<Template.Component>;
		};
		let { components } = await Args.apply<Template.Arg, Apply>(
			args,
			async (arg) => {
				if (arg === undefined) {
					return {};
				} else if (Template.Component.is(arg)) {
					return { components: { kind: "append" as const, value: arg } };
				} else if (Template.is(arg)) {
					return {
						components: { kind: "append" as const, value: arg.components },
					};
				} else {
					return unreachable();
				}
			},
		);

		// Normalize the components.
		components = (components ?? []).reduce<Array<Template.Component>>(
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
		argTemplates = argTemplates.filter((arg) => arg.components.length > 0);
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

	get components(): Array<Template.Component> {
		return this.#components;
	}
}

export namespace Template {
	export type Arg = undefined | Component | Template;

	export namespace Arg {
		export let is = (value: unknown): value is Arg => {
			return (
				value === undefined ||
				Component.is(value) ||
				Template.is(value) ||
				(value instanceof Array && value.every((value) => Arg.is(value)))
			);
		};
	}

	export type Component = string | Artifact;

	export namespace Component {
		export let is = (value: unknown): value is Component => {
			return typeof value === "string" || Artifact.is(value);
		};
	}
}
