import "./syscall";
import {
	Artifact,
	ArtifactHash,
	addArtifact,
	getArtifact,
	isArtifact,
} from "./artifact";
import { Dependency } from "./dependency";
import { Directory } from "./directory";
import { File } from "./file";
import { Placeholder } from "./placeholder";
import { Symlink } from "./symlink";
import { Template } from "./template";

export type Value =
	| (undefined | null)
	| boolean
	| number
	| string
	| Artifact
	| Placeholder
	| Template
	| Array<Value>
	| { [key: string]: Value };

export let isValue = (value: unknown): value is Value => {
	return (
		value === undefined ||
		value === null ||
		typeof value === "boolean" ||
		typeof value === "number" ||
		typeof value === "string" ||
		value instanceof Directory ||
		value instanceof File ||
		value instanceof Symlink ||
		value instanceof Dependency ||
		value instanceof Template ||
		Array.isArray(value) ||
		typeof value === "object"
	);
};

export let serializeValue = async <T extends Value>(
	value: T,
): Promise<syscall.Value> => {
	if (value === undefined || value === null) {
		return {
			type: "null",
			value,
		};
	} else if (typeof value === "boolean") {
		return {
			type: "bool",
			value,
		};
	} else if (typeof value === "number") {
		return {
			type: "number",
			value,
		};
	} else if (typeof value === "string") {
		return {
			type: "string",
			value,
		};
	} else if (isArtifact(value)) {
		return {
			type: "artifact",
			value: (await addArtifact(value)).toString(),
		};
	} else if (value instanceof Placeholder) {
		return {
			type: "placeholder",
			value: await value.serialize(),
		};
	} else if (value instanceof Template) {
		return {
			type: "template",
			value: await value.serialize(),
		};
	} else if (Array.isArray(value)) {
		let serializedValue = await Promise.all(
			value.map((value) => serializeValue(value)),
		);
		return {
			type: "array",
			value: serializedValue,
		};
	} else if (typeof value === "object") {
		let serializedValue = Object.fromEntries(
			await Promise.all(
				Object.entries(value).map(async ([key, value]) => [
					key,
					await serializeValue(value),
				]),
			),
		);
		return {
			type: "map",
			value: serializedValue,
		};
	} else {
		throw new Error("Failed to serialize the value.");
	}
};

export let deserializeValue = async (value: syscall.Value): Promise<Value> => {
	switch (value.type) {
		case "null": {
			return value.value;
		}
		case "bool": {
			return value.value;
		}
		case "number": {
			return value.value;
		}
		case "string": {
			return value.value;
		}
		case "artifact": {
			return await getArtifact(new ArtifactHash(value.value));
		}
		case "placeholder": {
			return await Placeholder.deserialize(value.value);
		}
		case "template": {
			return await Template.deserialize(value.value);
		}
		case "array": {
			return await Promise.all(
				value.value.map((value) => deserializeValue(value)),
			);
		}
		case "map": {
			return Object.fromEntries(
				await Promise.all(
					Object.entries(value.value).map(async ([key, value]) => [
						key,
						await deserializeValue(value),
					]),
				),
			);
		}
	}
};
