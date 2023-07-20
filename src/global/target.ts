import { assert as assert_ } from "./assert.ts";
import { Block } from "./block.ts";
import { json } from "./encoding.ts";
import { env } from "./env.ts";
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
	module: syscall.Module;
	name: string;
};

export let target = <
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
>(
	arg: TargetArg<A, R>,
) => {
	// Create the target.
	assert_(arg.module.kind === "normal");
	let target = Target.fromSyscall<A, R>(
		syscall.target.new({
			package: arg.module.value.package,
			modulePath: arg.module.value.modulePath,
			name: arg.name,
			env: {},
			args: [],
		}),
	);
	target.f = arg.f;

	// Register the target.
	let key = json.encode({ module: arg.module, name: arg.name });
	assert_(targets[key] === undefined);
	targets[key] = target;

	return target;
};

export let entrypoint = async <A extends Array<Value>, R extends Value>(
	f: (...args: A) => MaybePromise<R>,
	syscallEnv: Record<string, syscall.Value>,
	syscallArgs: Array<syscall.Value>,
): Promise<syscall.Value> => {
	// Set the env.
	env.value = Object.fromEntries(
		Object.entries(syscallEnv).map(([key, value]) => [
			key,
			Value.fromSyscall(value),
		]),
	);

	// Get the args.
	let args = syscallArgs.map((value) => Value.fromSyscall(value)) as A;

	// Call the target's function.
	let output = await f(...args);

	// Get the output.
	let syscallOutput = Value.toSyscall(output);

	return syscallOutput;
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
	modulePath: Subpath.Arg;
	name: string;
	env?: Record<string, Value>;
	args?: A;
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
	block: Block;
	package: Block;
	modulePath: Subpath;
	name: string;
	env?: Record<string, Value>;
	args?: A;

	static new<A extends Array<Value> = Array<Value>, R extends Value = Value>(
		arg: NewArg<A, R>,
	): Target<A, R> {
		let env_ = Object.fromEntries(
			Object.entries(arg.env ?? {}).map(([key, value]) => [
				key,
				Value.toSyscall(value),
			]),
		);
		let args_ = (arg.args ?? []).map((value) => Value.toSyscall(value));
		let target = Target.fromSyscall<A, R>(
			syscall.target.new({
				package: arg.target.package.toSyscall(),
				modulePath: arg.target.modulePath.toSyscall(),
				name: arg.target.name,
				env: env_,
				args: args_,
			}),
		);
		target.f = arg.target.f;
		return target;
	}

	constructor(arg: ConstructorArg<A, R>) {
		super();

		this.f = arg.f;
		this.block = arg.block;
		this.package = arg.package;
		this.modulePath = subpath(arg.modulePath);
		this.name = arg.name;
		this.env = arg.env;
		this.args = arg.args;

		// Proxy this object so that it is callable.
		return new Proxy(this, {
			apply: async (target, _, args) => {
				let target_ = Target.new({
					target,
					args: (await Promise.all(args.map(resolve))) as A,
					env: env.value,
				});
				let syscallOutput = await syscall.operation.evaluation(
					Operation.toSyscall(target_ as Operation),
				);
				let output = Value.fromSyscall(syscallOutput) as R;
				return output;
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

	toSyscall(): syscall.Target {
		let block = this.block.toSyscall();
		let package_ = this.package.toSyscall();
		let modulePath = this.modulePath.toString();
		let name = this.name;
		let env = this.env
			? Object.fromEntries(
					Object.entries(this.env).map(([key, value]) => [
						key,
						Value.toSyscall(value),
					]),
			  )
			: undefined;
		let args = this.args
			? this.args.map((arg) => Value.toSyscall(arg))
			: undefined;
		return {
			block,
			package: package_,
			modulePath,
			name,
			env,
			args,
		};
	}

	static fromSyscall<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	>(target: syscall.Target): Target<A, R> {
		let block = Block.fromSyscall(target.block);
		let package_ = Block.fromSyscall(target.package);
		let modulePath = target.modulePath;
		let name = target.name;
		let env =
			target.env !== undefined
				? Object.fromEntries(
						Object.entries(target.env).map(([key, value]) => [
							key,
							Value.fromSyscall(value),
						]),
				  )
				: undefined;
		let args =
			target.args !== undefined
				? (target.args.map((arg) => Value.fromSyscall(arg)) as A)
				: undefined;
		return new Target({
			block,
			package: package_,
			modulePath,
			name,
			env,
			args,
		});
	}
}
