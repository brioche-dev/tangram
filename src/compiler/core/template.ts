import {
	Artifact,
	ArtifactHash,
	addArtifact,
	getArtifact,
	isArtifact,
} from "./artifact.ts";
import { Placeholder } from "./placeholder.ts";
import { MaybeArray, MaybePromise } from "./util.ts";

export type TemplateLike = Template | MaybeArray<TemplateComponent>;

export let template = async (
	strings: TemplateStringsArray,
	...placeholders: Array<MaybePromise<Template | MaybeArray<TemplateComponent>>>
): Promise<Template> => {
	let resolvedPlaceholders = await Promise.all(placeholders);
	let components = [];
	for (let i = 0; i < strings.length - 1; i++) {
		let string = strings[i];
		let placeholder = resolvedPlaceholders[i];
		components.push(string);
		if (placeholder instanceof Template) {
			components.push(...placeholder.components);
		} else if (Array.isArray(placeholder)) {
			components.push(...placeholder);
		} else {
			components.push(placeholder);
		}
	}
	components.push(strings[strings.length - 1]);
	return new Template(components);
};

export { template as t };

export class Template {
	components: Array<TemplateComponent>;

	constructor(arg: TemplateLike) {
		if (arg instanceof Template) {
			this.components = [...arg.components];
		} else if (Array.isArray(arg)) {
			this.components = arg;
		} else {
			this.components = [arg];
		}
	}

	async serialize(): Promise<syscall.Template> {
		let components = await Promise.all(
			this.components.map(
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
}

export type TemplateComponent = string | Artifact | Placeholder;

export let serializeTemplateComponent = async (
	component: TemplateComponent,
): Promise<syscall.TemplateComponent> => {
	if (typeof component === "string") {
		return {
			type: "string",
			value: component,
		};
	} else if (isArtifact(component)) {
		return {
			type: "artifact",
			value: (await addArtifact(component)).toString(),
		};
	} else if (component instanceof Placeholder) {
		return {
			type: "placeholder",
			value: await component.serialize(),
		};
	} else {
		throw new Error("Invalid template component.");
	}
};

export let deserializeTemplateComponent = async (
	component: syscall.TemplateComponent,
): Promise<TemplateComponent> => {
	switch (component.type) {
		case "string": {
			return await component.value;
		}
		case "artifact": {
			return await getArtifact(new ArtifactHash(component.value));
		}
		case "placeholder": {
			return await Placeholder.deserialize(component.value);
		}
	}
};
