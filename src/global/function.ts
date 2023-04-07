import { assert } from "./assert.ts";
import { call } from "./call.ts";
import { env } from "./env.ts";
import { PackageInstance } from "./package.ts";
import { MaybePromise, Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { Value } from "./value.ts";

export let function_ = <
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
>(
	f: (...args: A) => MaybePromise<R>,
): Function<A, R> => {
	// Get the function's caller.
	let { module, line } = syscall.caller();

	// Get the function's package instance hash.
	assert(module.kind === "normal");
	let packageInstanceHash = module.value.packageInstanceHash;

	// Get the function's name.
	let name;
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

	return new Function({
		packageInstanceHash,
		name,
		f,
	});
};

type ConstructorArgs<A extends Array<Value>, R extends Value> = {
	packageInstanceHash: PackageInstance.Hash;
	name: string;
	f?: (...args: A) => MaybePromise<R>;
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
	packageInstanceHash: PackageInstance.Hash;
	name: string;
	f?: (...args: A) => MaybePromise<R>;

	constructor(args: ConstructorArgs<A, R>) {
		super();

		this.packageInstanceHash = args.packageInstanceHash;
		this.name = args.name;
		this.f = args.f;

		// Proxy this object so that it is callable.
		return new Proxy(this, {
			apply: async (target, _, args) => {
				let resolvedArgs = await Promise.all(args.map(resolve));
				return await call({
					function: target,
					args: resolvedArgs as A,
				});
			},
		});
	}

	static isFunction(value: unknown): value is Function {
		return value instanceof Function;
	}

	toSyscall(): syscall.Function {
		let packageInstanceHash = this.packageInstanceHash;
		let name = this.name?.toString();
		return {
			packageInstanceHash,
			name,
		};
	}

	static fromSyscall<A extends Array<Value>, R extends Value>(
		function_: syscall.Function,
	): Function<A, R> {
		let packageInstanceHash = function_.packageInstanceHash;
		let name = function_.name;
		return new Function({
			packageInstanceHash,
			name,
		});
	}

	async run(
		syscallEnv: { [key: string]: syscall.Value },
		syscallArgs: Array<syscall.Value>,
	): Promise<syscall.Value> {
		// Set the env.
		for (let [key, value] of Object.entries(syscallEnv)) {
			env.set(key, Value.fromSyscall(value));
		}

		// Get the args.
		let args = syscallArgs.map(Value.fromSyscall) as A;

		// Call the function.
		assert(this.f);
		let output = await this.f(...args);

		return Value.toSyscall(output);
	}
}
