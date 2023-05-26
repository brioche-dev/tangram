import { assert as assert_ } from "./assert.ts";
import { json } from "./encoding.ts";
import { env as globalEnv } from "./env.ts";
import { Operation } from "./operation.ts";
import { Package } from "./package.ts";
import { Subpath, subpath } from "./path.ts";
import { MaybePromise, Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { Value } from "./value.ts";

export let registry: Record<string, Function<any, any>> = {};

type FunctionArg<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
> = {
	f: (...args: A) => MaybePromise<R>;
	module: syscall.Module;
	name: string;
};

export let function_ = async <
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
>(
	arg: FunctionArg<A, R>,
) => {
	// Create the function.
	assert_(arg.module.kind === "normal");
	let function_ = Function.fromSyscall<A, R>(
		await syscall.function.new({
			packageHash: arg.module.value.packageHash,
			modulePath: arg.module.value.modulePath,
			name: arg.name,
			env: {},
			args: [],
		}),
	);
	function_.f = arg.f;

	// Add the function to the registry.
	let key = json.encode({ module: arg.module, name: arg.name });
	assert_(registry[key] === undefined);
	registry[key] = function_;

	return function_;
};

export let test = async (arg: FunctionArg<[], undefined>) => {
	return await function_(arg);
};

export let entrypoint = async <A extends Array<Value>, R extends Value>(
	f: (...args: A) => MaybePromise<R>,
	syscallEnv: Record<string, syscall.Value>,
	syscallArgs: Array<syscall.Value>,
): Promise<syscall.Value> => {
	// Set the env.
	globalEnv.value = Object.fromEntries(
		Object.entries(syscallEnv).map(([key, value]) => [
			key,
			Value.fromSyscall(value),
		]),
	);

	// Get the args.
	let args = syscallArgs.map((value) => Value.fromSyscall(value)) as A;

	// Call the function.
	let output = await f(...args);

	// Get the output.
	let syscallOutput = Value.toSyscall(output);

	return syscallOutput;
};

type NewArg<A extends Array<Value> = Array<Value>, R extends Value = Value> = {
	function: Function<A, R>;
	env?: Record<string, Value>;
	args?: A;
};

type ConstructorArg<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
> = {
	f?: (...args: A) => MaybePromise<R>;
	hash: Operation.Hash;
	packageHash: Package.Hash;
	modulePath: Subpath.Arg;
	name: string;
	env?: Record<string, Value>;
	args?: A;
};

export interface Function<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
> {
	(...args: { [K in keyof A]: Unresolved<A[K]> }): Promise<R>;
}

export class Function<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
> extends globalThis.Function {
	f?: (...args: A) => MaybePromise<R>;
	hash: Operation.Hash;
	packageHash: Package.Hash;
	modulePath: Subpath;
	name: string;
	env?: Record<string, Value>;
	args?: A;

	static async new<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	>(arg: NewArg<A, R>): Promise<Function<A, R>> {
		let env_ = Object.fromEntries(
			Object.entries(arg.env ?? {}).map(([key, value]) => [
				key,
				Value.toSyscall(value),
			]),
		);
		let args_ = (arg.args ?? []).map((value) => Value.toSyscall(value));
		let function_ = Function.fromSyscall<A, R>(
			await syscall.function.new({
				packageHash: arg.function.packageHash,
				modulePath: arg.function.modulePath.toSyscall(),
				name: arg.function.name,
				env: env_,
				args: args_,
			}),
		);
		function_.f = arg.function.f;
		return function_;
	}

	constructor(arg: ConstructorArg<A, R>) {
		super();

		this.f = arg.f;
		this.hash = arg.hash;
		this.packageHash = arg.packageHash;
		this.modulePath = subpath(arg.modulePath);
		this.name = arg.name;
		this.env = arg.env;
		this.args = arg.args;

		// Proxy this object so that it is callable.
		return new Proxy(this, {
			apply: async (target, _, args) => {
				let function_ = await Function.new({
					function: target,
					args: (await Promise.all(args.map(resolve))) as A,
				});
				let syscallOutput = await syscall.operation.run(
					Operation.toSyscall(function_ as Operation),
				);
				let output = Value.fromSyscall(syscallOutput) as R;
				return output;
			},
		});
	}

	static is(value: unknown): value is Function {
		return value instanceof Function;
	}

	static expect(value: unknown): Function {
		assert_(Function.is(value));
		return value;
	}

	static assert(value: unknown): asserts value is Function {
		assert_(Function.is(value));
	}

	toSyscall(): syscall.Function {
		let hash = this.hash;
		let packageHash = this.packageHash;
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
			hash,
			packageHash,
			modulePath,
			name,
			env,
			args,
		};
	}

	static fromSyscall<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	>(function_: syscall.Function): Function<A, R> {
		let hash = function_.hash;
		let packageHash = function_.packageHash;
		let modulePath = function_.modulePath;
		let name = function_.name;
		let env =
			function_.env !== undefined
				? Object.fromEntries(
						Object.entries(function_.env).map(([key, value]) => [
							key,
							Value.fromSyscall(value),
						]),
				  )
				: undefined;
		let args =
			function_.args !== undefined
				? (function_.args.map((arg) => Value.fromSyscall(arg)) as A)
				: undefined;
		return new Function({
			hash,
			packageHash,
			modulePath,
			name,
			env,
			args,
		});
	}
}
