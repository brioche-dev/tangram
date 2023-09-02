import { assert as assert_ } from "./assert.ts";
import { Build } from "./build.ts";
import { json } from "./encoding.ts";
import { env } from "./env.ts";
import { Id } from "./id.ts";
import { Module } from "./module.ts";
import { Subpath, subpath } from "./path.ts";
import { MaybePromise, Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { Value } from "./value.ts";

export let targets: Record<string, Function> = {};

export type TargetArg<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
> = {
	function: (...args: A) => MaybePromise<R>;
	module: Module;
	name: string;
};

export let target = async <
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
>(
	arg: TargetArg<A, R>,
): Promise<Target<A, R>> => {
	// Create the target.
	let target = new Target({
		package: arg.module.package,
		path: subpath(arg.module.path),
		name: arg.name,
		env: {},
		args: [],
	}) as unknown as Target<A, R>;

	// Register the target function.
	let key = json.encode({
		package: arg.module.package,
		path: arg.module.path,
		name: arg.name,
	});
	assert_(targets[key] === undefined);
	targets[key] = arg.function;

	return target;
};

type ConstructorArg<A extends Array<Value> = Array<Value>> = {
	package: Id;
	path: Subpath.Arg;
	name: string;
	env: Record<string, Value>;
	args: A;
};

export interface Target<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
> extends globalThis.Function {
	(...args: { [K in keyof A]: Unresolved<A[K]> }): Promise<R>;
}

export class Target<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
> extends globalThis.Function {
	#id: Id | undefined;
	#data: Target.Data | undefined;

	constructor(arg: ConstructorArg<A>) {
		super();

		// Set the state.
		this.#data = {
			package: arg.package,
			path: subpath(arg.path),
			name: arg.name,
			env: arg.env,
			args: arg.args,
		};

		// Proxy this object so that it is callable.
		return new Proxy(this, {
			apply: async (target, _, args) => {
				await this.load();
				let target_ = new Target({
					package: target.#data!.package,
					path: target.#data!.path,
					name: target.#data!.name,
					args: (await Promise.all(args.map(resolve))) as A,
					env: env.get(),
				});
				return await syscall.build.output(target_ as Build);
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

	async load(): Promise<void> {
		if (!this.#data) {
			this.#data = ((await syscall.value.load(this as Target)) as Target).#data;
		}
	}

	async store(): Promise<void> {
		if (!this.#id) {
			this.#id = ((await syscall.value.store(this as Target)) as Target).#id;
		}
	}

	async path(): Promise<Subpath> {
		return this.#data!.path;
	}

	async name_(): Promise<string> {
		return this.#data!.name;
	}

	async env(): Promise<Record<string, Value>> {
		return this.#data!.env;
	}

	async args(): Promise<A> {
		return this.#data!.args as A;
	}
}

export namespace Target {
	export type Data = {
		package: Id;
		path: Subpath;
		name: string;
		env: Record<string, Value>;
		args: Array<Value>;
	};
}
