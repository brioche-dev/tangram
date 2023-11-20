import { assert as assert_ } from "./assert.ts";
import { Unresolved, resolve } from "./resolve.ts";
import { Template, template } from "./template.ts";
import { MaybeNestedArray, flatten } from "./util.ts";
import { Value } from "./value.ts";

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
		if (arg.kind === "array_prepend" || arg.kind === "array_append") {
			return new Mutation({ kind: arg.kind, values: flatten(arg.values) });
		} else if (
			arg.kind === "template_prepend" ||
			arg.kind === "template_append"
		) {
			return new Mutation({
				kind: arg.kind,
				template: await template(arg.template),
				separator: arg.separator,
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
				template: T extends Template ? Template.Arg : never;
				separator?: string | undefined;
		  }
		| {
				kind: "template_append";
				template: T extends Template ? Template.Arg : never;
				separator?: string | undefined;
		  };

	export type Inner =
		| { kind: "unset" }
		| { kind: "set"; value: Value }
		| { kind: "set_if_unset"; value: Value }
		| {
				kind: "array_prepend";
				values: Array<Value>;
		  }
		| {
				kind: "array_append";
				values: Array<Value>;
		  }
		| {
				kind: "template_prepend";
				template: Template;
				separator: string | undefined;
		  }
		| {
				kind: "template_append";
				template: Template;
				separator: string | undefined;
		  };
}
