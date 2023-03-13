import { Artifact } from "./artifact";
import { Directory } from "./directory";
import { File } from "./file";
import { Placeholder } from "./placeholder";
import { Reference } from "./reference";
import { Symlink } from "./symlink";
import { Template } from "./template";
import { Value, nullish } from "./value";

export type Unresolved<T extends Value> = T extends
	| nullish
	| boolean
	| number
	| string
	| Artifact
	| Placeholder
	| Template
	? MaybeThunk<MaybePromise<T>>
	: T extends Array<infer U extends Value>
	? MaybeThunk<MaybePromise<Array<Unresolved<U>>>>
	: T extends { [key: string]: Value }
	? MaybeThunk<MaybePromise<{ [K in keyof T]: Unresolved<T[K]> }>>
	: never;

export type Resolved<T extends Unresolved<Value>> = T extends
	| nullish
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
	: T extends (() => infer U extends Unresolved<Value>)
	? Resolved<U>
	: T extends Promise<infer U extends Unresolved<Value>>
	? Resolved<U>
	: never;

export type MaybeThunk<T> = T | (() => T);

export type MaybePromise<T> = T | PromiseLike<T>;

export type MaybeArray<T> = T | Array<T>;

export let resolve = async <T extends Unresolved<Value>>(
	value: T,
): Promise<Resolved<T>> => {
	value = await value;
	if (
		value === undefined ||
		value === null ||
		typeof value === "boolean" ||
		typeof value === "number" ||
		typeof value === "string" ||
		value instanceof Directory ||
		value instanceof File ||
		value instanceof Symlink ||
		value instanceof Reference ||
		value instanceof Placeholder ||
		value instanceof Template
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
	} else if (typeof value === "function") {
		return (await resolve(value())) as Resolved<T>;
	} else {
		throw new Error("Invalid value to resolve.");
	}
};
