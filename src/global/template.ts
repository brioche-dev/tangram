import "./syscall";
import {
	Artifact,
	ArtifactHash,
	addArtifact,
	getArtifact,
	isArtifact,
} from "./artifact";
import { Placeholder } from "./placeholder";
import { MaybeArray, MaybePromise } from "./util";

export type TemplateLike = Template | MaybeArray<TemplateComponent>;

export let t = async (
	strings: TemplateStringsArray,
	...placeholders: Array<MaybePromise<Template | MaybeArray<TemplateComponent>>>
): Promise<Template> => {
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
	components: MaybeArray<
		MaybePromise<Template | MaybeArray<TemplateComponent>>
	>,
): Promise<Template> => {
	let resolvedComponents = await Promise.all(
		Array.isArray(components) ? components : [components],
	);
	let flattenedComponents = [];
	for (let component of resolvedComponents) {
		if (component instanceof Template) {
			flattenedComponents.push(...component.components);
		} else if (Array.isArray(component)) {
			flattenedComponents.push(...component);
		} else {
			flattenedComponents.push(component);
		}
	}
	return new Template(flattenedComponents);
};

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
