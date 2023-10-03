import { assert as assert_, todo } from "./assert.ts";
import { json } from "./encoding.ts";
import { Module } from "./module.ts";
import { MaybePromise, Unresolved } from "./resolve.ts";
import { Task } from "./task.ts";
import { Value } from "./value.ts";

export let functions: Record<string, Function> = {};

export type TargetArg<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
> = {
	function: (...args: A) => MaybePromise<R>;
	module: Module;
	name: string;
};

export let target = <
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
>(
	arg: TargetArg<A, R>,
): Target<A, R> => {
	// Create the target.
	let target = new Target<A, R>({
		module: arg.module,
		name: arg.name,
	});

	// Register the target function.
	let key = json.encode({
		module: {
			package: arg.module.package.handle().expectId(),
			path: arg.module.path,
		},
		name: arg.name,
	});
	assert_(functions[key] === undefined);
	functions[key] = arg.function;

	return target;
};

export interface Target<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
> extends globalThis.Function {
	(...args: { [K in keyof A]: Unresolved<A[K]> }): Promise<R>;
}

type ConstructorArg = {
	module: Module;
	name: string;
};

export class Target<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
> extends globalThis.Function {
	#module: Module;
	#name: string;

	constructor(arg: ConstructorArg) {
		super();

		this.#module = arg.module;
		this.#name = arg.name;

		// Proxy this object so that it is callable.
		return new Proxy(this, {
			apply: async (target, _, args) => {
				let task = await Task.new({
					host: "js-js",
					executable: target.#module.path,
					package: target.#module.package,
					target: target.#name,
					args,
					env: todo(),
				});
				return await task.run();
			},
		});
	}

	static is(value: unknown): value is Target {
		return value instanceof Target;
	}

	static expect(value: unknown): Target {
		assert_(Target.is(value));
		return value;
	}

	static assert(value: unknown): asserts value is Target {
		assert_(Target.is(value));
	}
}
