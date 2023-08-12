import { assert as assert_ } from "./assert.ts";
import { Block } from "./block.ts";
import { json } from "./encoding.ts";
import { env } from "./env.ts";
import { Id } from "./id.ts";
import { Module } from "./module.ts";
import { Operation } from "./operation.ts";
import { Subpath, subpath } from "./path.ts";
import { MaybePromise, Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { Value } from "./value.ts";

export let targets: Record<string, Target<any, any>> = {};

export type TargetArg<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
> = {
	f: (...args: A) => MaybePromise<R>;
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
	let target = (await syscall.target.new({
		package: arg.module.package(),
		path: subpath(arg.module.path()),
		name: arg.name,
		env: {},
		args: [],
	})) as Target<A, R>;
	target.f = arg.f;

	// Register the target.
	let key = json.encode({
		package: arg.module.package().id(),
		path: arg.module.path(),
		name: arg.name,
	});
	assert_(targets[key] === undefined);
	targets[key] = target;

	return target;
};

type NewArg<A extends Array<Value> = Array<Value>, R extends Value = Value> = {
	target: Target<A, R>;
	env?: Record<string, Value>;
	args?: A;
};

type ConstructorArg<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
> = {
	f?: (...args: A) => MaybePromise<R>;
	block: Block;
	package: Block;
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
	f?: (...args: A) => MaybePromise<R>;
	#block: Block;
	#package: Block;
	#path: Subpath;
	#name_: string;
	#env: Record<string, Value>;
	#args: A;

	constructor(arg: ConstructorArg<A, R>) {
		super();

		this.f = arg.f;
		this.#block = arg.block;
		this.#package = arg.package;
		this.#path = subpath(arg.path);
		this.#name_ = arg.name;
		this.#env = arg.env;
		this.#args = arg.args;

		// Proxy this object so that it is callable.
		return new Proxy(this, {
			apply: async (target, _, args) => {
				let target_ = await Target.new({
					target,
					args: (await Promise.all(args.map(resolve))) as A,
					env: env.get(),
				});
				return await syscall.operation.evaluate(target_ as Operation);
			},
		});
	}

	static async new<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	>(arg: NewArg<A, R>): Promise<Target<A, R>> {
		let target = (await syscall.target.new({
			package: arg.target.#package,
			path: arg.target.#path,
			name: arg.target.#name_,
			env: arg.env ?? {},
			args: arg.args ?? [],
		})) as Target<A, R>;
		target.f = arg.target.f;
		return target;
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

	id(): Id {
		return this.block().id();
	}

	block(): Block {
		return this.#block;
	}

	package(): Block {
		return this.#package;
	}

	path(): Subpath {
		return this.#path;
	}

	name_(): string {
		return this.#name_;
	}

	env(): Record<string, Value> {
		return this.#env;
	}

	args(): Array<Value> {
		return this.#args;
	}
}
