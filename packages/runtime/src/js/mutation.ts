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

type MaybeMutationMap<T extends Value = Value> = T extends
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
	| Package
	| Target
	| Array<infer _U extends Value>
	? T
	: T extends { [key: string]: Value }
	? MutationMap<T>
	: never;

export type MutationMap<T extends { [key: string]: Value }> = {
	[K in keyof T]?: MaybeNestedArray<MaybeMutation<T[K]>>;
};

export type MaybeMutation<T extends Value = Value> = T | Mutation<T>;

export type MaybeNestedArray<T> = T | Array<MaybeNestedArray<T>>;

export type MaybePromise<T> = T | Promise<T>;

export let apply = async <
	A extends Value = Value,
	R extends { [key: string]: Value } = { [key: string]: Value },
>(
	args: Args<A>,
	map: (arg: A) => Promise<MaybeNestedArray<MutationMap<R>>>,
): Promise<Partial<R>> => {
	return flatten(
		await Promise.all(
			flatten(await Promise.all(args.map(resolve))).map((arg) => map(arg as A)),
		),
	).reduce(async (object, mutations) => {
		for (let [key, mutation] of Object.entries(mutations)) {
			await mutate(object, key, mutation);
		}
		return object;
	}, {});
};

/** Create a mutation. */
export async function mutation<T extends Value = Value>(
	arg: Unresolved<Mutation.Arg<T>>,
): Promise<Mutation<T>> {
	return await Mutation.new(arg);
}

export class Mutation<T extends Value = Value> {
	#inner: Mutation.Inner;

	constructor(inner: Mutation.Inner) {
		this.#inner = inner;
	}

	static async new<T extends Value = Value>(
		unresolvedArg: Unresolved<Mutation.Arg<T>>,
	): Promise<Mutation<T>> {
		let arg = await resolve(unresolvedArg);
		if (arg.kind === "prepend_array" || arg.kind === "append_array") {
			return new Mutation({ kind: arg.kind, value: flatten(arg.value) });
		} else if (
			arg.kind === "prepend_template" ||
			arg.kind === "append_template"
		) {
			return new Mutation({
				kind: arg.kind,
				value: await template(arg.value),
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
				kind: "prepend_array";
				value: T extends Array<infer U> ? MaybeNestedArray<U> : never;
		  }
		| {
				kind: "append_array";
				value: T extends Array<infer U> ? MaybeNestedArray<U> : never;
		  }
		| {
				kind: "prepend_template";
				value: T extends Template.Arg ? Template.Arg : never;
				separator: Template.Arg;
		  }
		| {
				kind: "append_template";
				value: T extends Template.Arg ? Template.Arg : never;
				separator: Template.Arg;
		  };

	export type Inner =
		| { kind: "unset" }
		| { kind: "set"; value: Value }
		| { kind: "set_if_unset"; value: Value }
		| {
				kind: "prepend_array";
				value: Array<Value>;
		  }
		| {
				kind: "append_array";
				value: Array<Value>;
		  }
		| {
				kind: "prepend_template";
				value: Template;
				separator: Template;
		  }
		| {
				kind: "append_template";
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
	} else if (mutation.inner.kind === "prepend_array") {
		if (!(key in object)) {
			object[key] = [];
		}
		let array = object[key];
		assert(array instanceof Array);
		array.unshift(...flatten(mutation.inner.value));
	} else if (mutation.inner.kind === "append_array") {
		if (!(key in object)) {
			object[key] = [];
		}
		let array = object[key];
		assert(array instanceof Array);
		array.push(...flatten(mutation.inner.value));
	} else if (mutation.inner.kind === "prepend_template") {
		if (!(key in object)) {
			object[key] = await template();
		}
		let t = object[key];
		assert(t instanceof Template);
		t = await Template.join(mutation.inner.separator, mutation.inner.value, t);
	} else if (mutation.inner.kind === "append_template") {
		if (!(key in object)) {
			object[key] = await template();
		}
		let t = object[key];
		assert(t instanceof Template);
		t = await Template.join(mutation.inner.separator, t, mutation.inner.value);
	}
};
