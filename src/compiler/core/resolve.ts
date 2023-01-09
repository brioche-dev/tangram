import { Artifact } from "./artifact.ts";
import { Dependency } from "./dependency.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Placeholder } from "./placeholder.ts";
import { Symlink } from "./symlink.ts";
import { Template } from "./template.ts";
import { MaybePromise } from "./util.ts";
import { Value } from "./value.ts";

export type Unresolved<T extends Value> = T extends
	| undefined
	| null
	| boolean
	| number
	| string
	| Artifact
	| Placeholder
	| Template
	? MaybePromise<T>
	: T extends Array<infer U extends Value>
	? MaybePromise<Array<Unresolved<U>>>
	: T extends { [key: string]: Value }
	? MaybePromise<{ [K in keyof T]: Unresolved<T[K]> }>
	: never;

export type Resolved<T extends Unresolved<Value>> = T extends
	| undefined
	| null
	| boolean
	| number
	| string
	| Artifact
	| Placeholder
	| Template
	? T
	: T extends Array<infer U extends Unresolved<Value>>
	? Array<Resolved<U>>
	: T extends { [key: string]: Unresolved<Value> }
	? { [K in keyof T]: Resolved<T[K]> }
	: T extends Promise<infer U extends Unresolved<Value>>
	? Resolved<U>
	: never;

export let resolve = async <T extends Unresolved<Value>>(
	value: T,
): Promise<Resolved<T>> => {
	let awaitedValue = await value;
	if (
		awaitedValue === undefined ||
		awaitedValue === null ||
		typeof awaitedValue === "boolean" ||
		typeof awaitedValue === "number" ||
		typeof awaitedValue === "string" ||
		awaitedValue instanceof Directory ||
		awaitedValue instanceof File ||
		awaitedValue instanceof Symlink ||
		awaitedValue instanceof Dependency ||
		awaitedValue instanceof Placeholder ||
		awaitedValue instanceof Template
	) {
		return awaitedValue as unknown as Resolved<T>;
	} else if (Array.isArray(awaitedValue)) {
		return (await Promise.all(
			awaitedValue.map((value) => resolve(value)),
		)) as Resolved<T>;
	} else {
		return Object.fromEntries(
			await Promise.all(
				Object.entries(awaitedValue).map(async ([key, value]) => [
					key,
					await resolve(value),
				]),
			),
		) as Resolved<T>;
	}
};
