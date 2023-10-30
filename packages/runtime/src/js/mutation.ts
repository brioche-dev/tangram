import { Artifact } from "./artifact.ts";
import { assert, assert as assert_ } from "./assert.ts";
import { Blob } from "./blob.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Package } from "./package.ts";
import { Unresolved, resolve } from "./resolve.ts";
import { Symlink } from "./symlink.ts";
import { Target } from "./target.ts";
import { Template, template } from "./template.ts";
import { Value } from "./value.ts";

export type Args<T extends Value = Value> = Array<
	Unresolved<MaybeNestedArray<MaybeMutationMap<T>>>
>;

export type MaybeMutationMap<T extends Value = Value> = T extends
	| undefined
	| boolean
	| number
	| string
	| Uint8Array
	| Blob
	| Directory
	| File
	| Symlink
	| Template
	| Mutation
	| Package
	| Target
	| Array<infer _U extends Value>
	? T
	: T extends { [key: string]: Value }
	? MutationMap<T>
	: never;

export type MutationMap<
	T extends { [key: string]: Value } = { [key: string]: Value },
> = {
	[K in keyof T]?: MaybeMutation<T[K]>;
};

export type MaybeMutation<T extends Value = Value> = T | Mutation<T>;

export type MaybeNestedArray<T> = T | Array<MaybeNestedArray<T>>;

export async function mutation<T extends Value = Value>(
	arg: Unresolved<Mutation.Arg<T>>,
): Promise<Mutation<T>> {
	return await Mutation.new(arg);
}

export let apply = async <
	A extends Value = Value,
	R extends { [key: string]: Value } = { [key: string]: Value },
>(
	args: Args<A>,
	map: (
		arg: Exclude<A, Array<Value>>,
	) => Promise<MaybeNestedArray<MutationMap<R>>>,
): Promise<Partial<R>> => {
	return flatten(
		await Promise.all(
			flatten(await Promise.all(args.map(resolve))).map((arg) =>
				map(arg as unknown as Exclude<A, Array<Value>>),
			),
		),
	).reduce(async (object, mutations) => {
		for (let [key, mutation] of Object.entries(mutations)) {
			await mutate(await object, key, mutation);
		}
		return object;
	}, Promise.resolve({}));
};

export class Mutation<T extends Value = Value> {
	#inner: Mutation.Inner;

	constructor(inner: Mutation.Inner) {
		this.#inner = inner;
	}

	static async new<T extends Value = Value>(
		unresolvedArg: Unresolved<Mutation.Arg<T>>,
	): Promise<Mutation<T>> {
		let arg = await resolve(unresolvedArg);
		if (arg.kind === "array_prepend" || arg.kind === "array_append") {
			return new Mutation({ kind: arg.kind, value: flatten(arg.values) });
		} else if (
			arg.kind === "template_prepend" ||
			arg.kind === "template_append"
		) {
			return new Mutation({
				kind: arg.kind,
				value: await template(arg.template),
				separator: await template(arg.separator),
			});
		} else if (arg.kind === "unset") {
			return new Mutation({ kind: "unset" });
		} else {
			return new Mutation({ kind: arg.kind, value: arg.value });
		}
	}

	/** Check if a value is a `tg.Mutation`. */
	static is(value: unknown): value is Mutation {
		return value instanceof Mutation;
	}

	/** Expect that a value is a `tg.Mutation`. */
	static expect(value: unknown): Mutation {
		assert_(Mutation.is(value));
		return value;
	}

	/** Assert that a value is a `tg.Mutation`. */
	static assert(value: unknown): asserts value is Mutation {
		assert_(Mutation.is(value));
	}

	get inner() {
		return this.#inner;
	}
}

export namespace Mutation {
	export type Arg<T extends Value = Value> =
		| { kind: "unset" }
		| { kind: "set"; value: T }
		| { kind: "set_if_unset"; value: T }
		| {
				kind: "array_prepend";
				values: T extends Array<infer U> ? MaybeNestedArray<U> : never;
		  }
		| {
				kind: "array_append";
				values: T extends Array<infer U> ? MaybeNestedArray<U> : never;
		  }
		| {
				kind: "template_prepend";
				template: T extends Template.Arg ? Template.Arg : never;
				separator?: Template.Arg;
		  }
		| {
				kind: "template_append";
				template: T extends Template.Arg ? Template.Arg : never;
				separator?: Template.Arg;
		  };

	export type Inner =
		| { kind: "unset" }
		| { kind: "set"; value: Value }
		| { kind: "set_if_unset"; value: Value }
		| {
				kind: "array_prepend";
				value: Array<Value>;
		  }
		| {
				kind: "array_append";
				value: Array<Value>;
		  }
		| {
				kind: "template_prepend";
				value: Template;
				separator: Template;
		  }
		| {
				kind: "template_append";
				value: Template;
				separator: Template;
		  };
}

export let flatten = <T>(value: MaybeNestedArray<T>): Array<T> => {
	// @ts-ignore
	return value instanceof Array ? value.flat(Infinity) : [value];
};

let mutate = async (
	object: { [key: string]: Value },
	key: string,
	mutation: MaybeMutation,
) => {
	if (!(mutation instanceof Mutation)) {
		object[key] = mutation;
	} else if (mutation.inner.kind === "unset") {
		delete object[key];
	} else if (mutation.inner.kind === "set") {
		object[key] = mutation.inner.value;
	} else if (mutation.inner.kind === "set_if_unset") {
		if (!(key in object)) {
			object[key] = mutation.inner.value;
		}
	} else if (mutation.inner.kind === "array_prepend") {
		if (!(key in object)) {
			object[key] = [];
		}
		let array = object[key];
		assert(array instanceof Array);
		array.unshift(...flatten(mutation.inner.value));
	} else if (mutation.inner.kind === "array_append") {
		if (!(key in object)) {
			object[key] = [];
		}
		let array = object[key];
		assert(array instanceof Array);
		array.push(...flatten(mutation.inner.value));
	} else if (mutation.inner.kind === "template_prepend") {
		if (!(key in object)) {
			object[key] = await template();
		}
		let value = object[key];
		assert(
			value === undefined ||
				typeof value === "string" ||
				Artifact.is(value) ||
				value instanceof Template,
		);
		object[key] = await Template.join(
			mutation.inner.separator,
			mutation.inner.value,
			value,
		);
	} else if (mutation.inner.kind === "template_append") {
		if (!(key in object)) {
			object[key] = await template();
		}
		let value = object[key];
		assert(
			value === undefined ||
				typeof value === "string" ||
				Artifact.is(value) ||
				value instanceof Template,
		);
		object[key] = await Template.join(
			mutation.inner.separator,
			value,
			mutation.inner.value,
		);
	}
};

