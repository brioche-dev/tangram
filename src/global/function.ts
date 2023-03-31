import { call } from "./call.ts";
import { context } from "./context.ts";
import { PackageInstanceHash } from "./package.ts";
import { MaybePromise, Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { assert } from "./util.ts";
import { Value, deserializeValue, serializeValue } from "./value.ts";

export let function_ = <
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
>(
	f: (...args: A) => MaybePromise<R>,
): Function<A, R> => {
	// Get the function's caller.
	let { packageInstanceHash, line } = syscall.caller();

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

export let isFunction = (value: unknown): value is Function => {
	return value instanceof Function;
};

type FunctionConstructorArgs<A extends Array<Value>, R extends Value> = {
	packageInstanceHash: PackageInstanceHash;
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
	packageInstanceHash: PackageInstanceHash;
	name: string;
	f?: (...args: A) => MaybePromise<R>;

	constructor(args: FunctionConstructorArgs<A, R>) {
		super();

		this.packageInstanceHash = args.packageInstanceHash;
		this.name = args.name;
		this.f = args.f;

		// Proxy this object so that it is callable.
		return new Proxy(this, {
			apply: (target, _, args) => target._call(...(args as A)),
		});
	}

	async serialize(): Promise<syscall.Function> {
		let packageInstanceHash = this.packageInstanceHash;
		let name = this.name?.toString();
		return {
			packageInstanceHash,
			name,
		};
	}

	static async deserialize<A extends Array<Value>, R extends Value>(
		function_: syscall.Function,
	): Promise<Function<A, R>> {
		let packageInstanceHash = function_.packageInstanceHash;
		let name = function_.name;
		return new Function({
			packageInstanceHash,
			name,
		});
	}

	async _call(...args: A): Promise<R> {
		let context_ = new Map(context);
		let resolvedArgs = await Promise.all(args.map(resolve));
		return await call({
			function: this,
			context: context_,
			args: resolvedArgs,
		});
	}

	async run(
		serializedContext: { [key: string]: syscall.Value },
		serializedArgs: Array<syscall.Value>,
	): Promise<syscall.Value> {
		// Deserialize and set the context.
		for (let [key, value] of Object.entries(serializedContext)) {
			context.set(key, await deserializeValue(value));
		}

		// Deserialize the args.
		let args = (await Promise.all(serializedArgs.map(deserializeValue))) as A;

		// Call the function.
		assert(this.f);
		let output = await this.f(...args);

		// Serialize the output.
		let serializedOutput = await serializeValue(output);

		return serializedOutput;
	}
}
