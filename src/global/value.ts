import { Artifact, addArtifact, getArtifact, isArtifact } from "./artifact";
import { Directory } from "./directory";
import { File } from "./file";
import { Placeholder } from "./placeholder";
import { Reference } from "./reference";
import { Symlink } from "./symlink";
import * as syscall from "./syscall";
import { Template } from "./template";

export type Value =
	| nullish
	| boolean
	| number
	| string
	| Artifact
	| Placeholder
	| Template
	| Array<Value>
	| { [key: string]: Value };

export type nullish = undefined | null;

export let isNullish = (value: unknown): value is nullish => {
	return value === undefined || value === null;
};

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
		value instanceof Reference ||
		value instanceof Template ||
		value instanceof Array ||
		typeof value === "object"
	);
};

export let serializeValue = async <T extends Value>(
	value: T,
): Promise<syscall.Value> => {
	if (value === undefined || value === null) {
		return {
			kind: "null",
			value,
		};
	} else if (typeof value === "boolean") {
		return {
			kind: "bool",
			value,
		};
	} else if (typeof value === "number") {
		return {
			kind: "number",
			value,
		};
	} else if (typeof value === "string") {
		return {
			kind: "string",
			value,
		};
	} else if (isArtifact(value)) {
		return {
			kind: "artifact",
			value: await addArtifact(value),
		};
	} else if (value instanceof Placeholder) {
		return {
			kind: "placeholder",
			value: await value.serialize(),
		};
	} else if (value instanceof Template) {
		return {
			kind: "template",
			value: await value.serialize(),
		};
	} else if (value instanceof Array) {
		let serializedValue = await Promise.all(
			value.map((value) => serializeValue(value)),
		);
		return {
			kind: "array",
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
			kind: "map",
			value: serializedValue,
		};
	} else {
		throw new Error("Failed to serialize the value.");
	}
};

export let deserializeValue = async (value: syscall.Value): Promise<Value> => {
	switch (value.kind) {
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
			return await getArtifact(value.value);
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
