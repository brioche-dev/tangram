import { Artifact, addArtifact, getArtifact, isArtifact } from "./artifact";
import { Placeholder } from "./placeholder";
import { Unresolved, resolve } from "./resolve";

export type TemplateLike = TemplateComponent | Template | Array<TemplateLike>;

export let t = async (
	strings: TemplateStringsArray,
	...placeholders: Array<Unresolved<TemplateLike>>
): Promise<Template> => {
	// Collect the strings and placeholders.
	let components = [];
	for (let i = 0; i < strings.length - 1; i++) {
		let string = strings[i];
		let placeholder = placeholders[i];
		components.push(string);
		components.push(placeholder);
	}
	components.push(strings[strings.length - 1]);

	return await template(components);
};

export let template = async (
	templateLike: Unresolved<TemplateLike>,
): Promise<Template> => {
	// Resolve the input.
	let resolvedTemplateLike = await resolve(templateLike);

	// Collect all components recursively.
	let components: Array<TemplateComponent> = [];
	let collectComponents = (templateLike: TemplateLike) => {
		if (templateLike instanceof Array) {
			templateLike.forEach(collectComponents);
		} else if (templateLike instanceof Template) {
			components.push(...templateLike.components());
		} else {
			components.push(templateLike);
		}
	};
	collectComponents(resolvedTemplateLike);

	return new Template(components);
};

export let isTemplate = (value: unknown): value is Template => {
	return value instanceof Template;
};

export class Template {
	#components: Array<TemplateComponent>;

	constructor(components: Array<TemplateComponent>) {
		this.#components = components;
	}

	async serialize(): Promise<syscall.Template> {
		let components = await Promise.all(
			this.#components.map(
				async (component) => await serializeTemplateComponent(component),
			),
		);
		return {
			components,
		};
	}

	static async deserialize(template: syscall.Template): Promise<Template> {
		return new Template(
			await Promise.all(
				template.components.map(
					async (component) => await deserializeTemplateComponent(component),
				),
			),
		);
	}

	components(): Array<TemplateComponent> {
		return [...this.#components];
	}

	render(f: (component: TemplateComponent) => string): string {
		return this.#components.map(f).join("");
	}
}

export type TemplateComponent = string | Artifact | Placeholder;

export let serializeTemplateComponent = async (
	component: TemplateComponent,
): Promise<syscall.TemplateComponent> => {
	if (typeof component === "string") {
		return {
			kind: "string",
			value: component,
		};
	} else if (isArtifact(component)) {
		return {
			kind: "artifact",
			value: await addArtifact(component),
		};
	} else if (component instanceof Placeholder) {
		return {
			kind: "placeholder",
			value: await component.serialize(),
		};
	} else {
		throw new Error("Invalid template component.");
	}
};

export let deserializeTemplateComponent = async (
	component: syscall.TemplateComponent,
): Promise<TemplateComponent> => {
	switch (component.kind) {
		case "string": {
			return await component.value;
		}
		case "artifact": {
			return await getArtifact(component.value);
		}
		case "placeholder": {
			return await Placeholder.deserialize(component.value);
		}
	}
};
