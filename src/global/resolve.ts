import { Artifact } from "./artifact.ts";
import { Blob } from "./blob.ts";
import { Block } from "./block.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Operation } from "./operation.ts";
import { Relpath, Subpath } from "./path.ts";
import { Placeholder } from "./placeholder.ts";
import { Resource } from "./resource.ts";
import { Symlink } from "./symlink.ts";
import { Target } from "./target.ts";
import { Task } from "./task.ts";
import { Template } from "./template.ts";
import { Value } from "./value.ts";

export type Unresolved<T extends Value> = MaybePromise<
	T extends
		| undefined
		| boolean
		| number
		| string
		| Uint8Array
		| Relpath
		| Subpath
		| Blob
		| Block
		| Artifact
		| Placeholder
		| Template
		| Operation
		? T
		: T extends Array<infer U extends Value>
		? Array<Unresolved<U>>
		: T extends { [key: string]: Value }
		? { [K in keyof T]: Unresolved<T[K]> }
		: never
>;

export type Resolved<T extends Unresolved<Value>> = T extends
	| undefined
	| boolean
	| number
	| string
	| Uint8Array
	| Relpath
	| Subpath
	| Blob
	| Block
	| Artifact
	| Placeholder
	| Template
	| Operation
	? T
	: T extends Promise<infer U extends Unresolved<Value>>
	? Resolved<U>
	: T extends Array<infer U extends Unresolved<Value>>
	? Array<Resolved<U>>
	: T extends { [key: string]: Unresolved<Value> }
	? { [K in keyof T]: Resolved<T[K]> }
	: never;

export type MaybePromise<T> = T | Promise<T>;

export let resolve = async <T extends Unresolved<Value>>(
	value: T,
): Promise<Resolved<T>> => {
	value = await value;
	if (
		value === undefined ||
		typeof value === "boolean" ||
		typeof value === "number" ||
		typeof value === "string" ||
		value instanceof Uint8Array ||
		value instanceof Relpath ||
		value instanceof Subpath ||
		value instanceof Blob ||
		value instanceof Directory ||
		value instanceof File ||
		value instanceof Symlink ||
		value instanceof Placeholder ||
		value instanceof Template ||
		value instanceof Resource ||
		value instanceof Target ||
		value instanceof Task
	) {
		return value as unknown as Resolved<T>;
	} else if (value instanceof Array) {
		return (await Promise.all(
			value.map((value) => resolve(value)),
		)) as Resolved<T>;
	} else if (typeof value === "object") {
		return Object.fromEntries(
			await Promise.all(
				Object.entries(value).map(async ([key, value]) => [
					key,
					await resolve(value),
				]),
			),
		) as Resolved<T>;
	} else {
		throw new Error("Invalid value to resolve.");
	}
};
