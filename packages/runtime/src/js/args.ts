import { assert } from "./assert.ts";
import { Unresolved, resolve } from "./resolve.ts";
import { Value } from "./value.ts";

export type Args<T extends Value> = Array<Unresolved<Args.MaybeNestedArray<T>>>;

export namespace Args {
	export type Mutation<T extends Value = Value> =
		| { kind: "unset" }
		| { kind: "set"; value: T }
		| { kind: "set_if_unset"; value: T }
		| {
				kind: "prepend";
				value: T extends Array<infer U> ? MaybeNestedArray<U> : never;
		  }
		| {
				kind: "append";
				value: T extends Array<infer U> ? MaybeNestedArray<U> : never;
		  };

	export type MaybeNestedArray<T> = T | Array<MaybeNestedArray<T>>;

	export type MutationObject<T extends { [key: string]: Value }> = {
		[K in keyof T]?: MaybeNestedArray<Mutation<T[K]>>;
	};

	export let apply = async <
		A extends Value,
		R extends { [key: string]: Value },
	>(
		args: Args<A>,
		map: (arg: A) => Promise<MaybeNestedArray<MutationObject<R>>>,
	): Promise<Partial<R>> => {
		return flatten(
			await Promise.all(
				flatten(await Promise.all(args.map(resolve))).map((arg) =>
					map(arg as A),
				),
			),
		).reduce((object, mutations) => {
			for (let [key, mutation] of Object.entries(mutations)) {
				mutate(object, key, mutation);
			}
			return object;
		}, {});
	};
}

export let flatten = <T>(value: Args.MaybeNestedArray<T>): Array<T> => {
	// @ts-ignore
	return value instanceof Array ? value.flat(Infinity) : [value];
};

let mutate = (
	object: { [key: string]: Value },
	key: string,
	mutation: Args.Mutation,
) => {
	if (mutation.kind === "unset") {
		delete object[key];
	} else if (mutation.kind === "set") {
		object[key] = mutation.value;
	} else if (mutation.kind === "set_if_unset") {
		if (!(key in object)) {
			object[key] = mutation.value;
		}
	} else if (mutation.kind === "prepend") {
		if (!(key in object)) {
			object[key] = [];
		}
		let array = object[key];
		assert(array instanceof Array);
		array.unshift(...flatten(mutation.value));
	} else if (mutation.kind === "append") {
		if (!(key in object)) {
			object[key] = [];
		}
		let array = object[key];
		assert(array instanceof Array);
		array.push(...flatten(mutation.value));
	}
};
