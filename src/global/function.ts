import { assert as assert_ } from "./assert.ts";
import { env } from "./env.ts";
import { Operation } from "./operation.ts";
import { Package } from "./package.ts";
import { Subpath, subpath } from "./path.ts";
import { MaybePromise, Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { Value } from "./value.ts";

export let function_ = async <
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
>(
	f: (...args: A) => MaybePromise<R>,
) => {
	return await Function.new(f);
};

export let call = async <A extends Array<Value>, R extends Value>(
	arg: Function.Arg<A, R>,
): Promise<R> => {
	let function_ = await Function.new(arg);
	let syscallOutput = await syscall.operation.run(
		Operation.toSyscall(function_ as Operation),
	);
	let output = Value.fromSyscall(syscallOutput) as R;
	return output;
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

	// Call the function.
	let output = await f(...args);

	// Get the output.
	let syscallOutput = Value.toSyscall(output);

	return syscallOutput;
};

type ConstructorArgs<A extends Array<Value>, R extends Value> = {
	f?: (...args: A) => MaybePromise<R>;
	hash: Operation.Hash;
	packageInstanceHash: Package.Instance.Hash;
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
	packageInstanceHash: Package.Instance.Hash;
	modulePath: Subpath;
	name: string;
	env?: Record<string, Value>;
	args?: A;

	static async new<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	>(arg: Function.Arg<A, R>): Promise<Function<A, R>> {
		let f: ((...args: A) => MaybePromise<R>) | undefined;
		let packageInstanceHash: Package.Instance.Hash;
		let modulePath: Subpath;
		let name: string;
		let env: Record<string, Value> | undefined;
		let args: A | undefined;

		if (arg instanceof globalThis.Function) {
			// Set the function.
			f = arg;

			// Get the function's caller.
			let { module, line } = syscall.stackFrame(2);

			// Get the function's package instance hash and module path.
			assert_(module.kind === "normal");
			packageInstanceHash = module.value.packageInstanceHash;
			modulePath = subpath(module.value.modulePath);

			// Get the function's name.
			if (line.startsWith("export default ")) {
				name = "default";
			} else if (line.startsWith("export let ")) {
				let exportName = line.match(/^export let ([a-zA-Z0-9]+)\b/)?.at(1);
				if (!exportName) {
					throw new Error("Invalid use of tg.function.");
				}
				name = exportName;
			} else {
				throw new Error("Invalid use of tg.function.");
			}
		} else {
			f = arg.function.f;
			packageInstanceHash = arg.function.packageInstanceHash;
			modulePath = subpath(arg.function.modulePath);
			name = arg.function.name;
			env = arg.env ?? {};
			args = arg.args ?? [];
		}

		let env_ =
			env !== undefined
				? Object.fromEntries(
						Object.entries(env).map(([key, value]) => [
							key,
							Value.toSyscall(value),
						]),
				  )
				: undefined;
		let args_ =
			args !== undefined
				? args.map((value) => Value.toSyscall(value))
				: undefined;

		let function_ = Function.fromSyscall<A, R>(
			await syscall.function.new({
				packageInstanceHash,
				modulePath: modulePath.toSyscall(),
				name,
				env: env_,
				args: args_,
			}),
		);
		function_.f = f;

		return function_;
	}

	constructor(args: ConstructorArgs<A, R>) {
		super();

		this.f = args.f;
		this.hash = args.hash;
		this.packageInstanceHash = args.packageInstanceHash;
		this.modulePath = subpath(args.modulePath);
		this.name = args.name;
		this.env = args.env;
		this.args = args.args;

		// Proxy this object so that it is callable.
		return new Proxy(this, {
			apply: async (target, _, args) => {
				return await call({
					function: target,
					args: (await Promise.all(args.map(resolve))) as A,
				});
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
		let packageInstanceHash = this.packageInstanceHash;
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
			packageInstanceHash,
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
		let packageInstanceHash = function_.packageInstanceHash;
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
			packageInstanceHash,
			modulePath,
			name,
			env,
			args,
		});
	}
}

export namespace Function {
	export type Arg<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	> = ((...args: A) => MaybePromise<R>) | ArgObject<A, R>;

	export type ArgObject<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	> = {
		function: Function<A, R>;
		env?: Record<string, Value>;
		args: A;
	};
}
