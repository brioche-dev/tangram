import { call } from "./call";
import { context } from "./context";
import { PackageInstanceHash } from "./package";
import { MaybePromise, Unresolved, resolve } from "./resolve";
import { assert } from "./util";
import { Value, deserializeValue, serializeValue } from "./value";

export let function_ = <
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
>(
	implementation: (...args: A) => MaybePromise<R>,
): Function<A, R> => {
	// Get the function's package instance hash and name.
	let packageInstanceHash = syscall("get_current_package_instance_hash");
	let name = syscall("get_current_export_name");

	return new Function({
		packageInstanceHash,
		name,
		implementation,
	});
};

export let isFunction = (value: unknown): value is Function => {
	return value instanceof Function;
};

type FunctionConstructorArgs<A extends Array<Value>, R extends Value> = {
	packageInstanceHash: PackageInstanceHash;
	name: string;
	implementation?: (...args: A) => MaybePromise<R>;
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
	implementation?: (...args: A) => MaybePromise<R>;

	constructor(args: FunctionConstructorArgs<A, R>) {
		super();

		this.packageInstanceHash = args.packageInstanceHash;
		this.name = args.name;
		this.implementation = args.implementation;

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
		reference: syscall.Function,
	): Promise<Function<A, R>> {
		let packageInstanceHash = reference.packageInstanceHash;
		let name = reference.name;
		return new Function({
			packageInstanceHash,
			name,
		});
	}

	async _call(...args: A): Promise<R> {
		let context_ = new Map(await context.entries());
		let resolvedArgs = await Promise.all(args.map(resolve));
		return await call({
			function: this,
			context: context_,
			args: resolvedArgs,
		});
	}

	async run(
		serializedArgs: Array<syscall.Value>,
		serializedContext: { [key: string]: syscall.Value },
	): Promise<syscall.Value> {
		// Ensure the implementation is available.
		assert(
			this.implementation,
			"This function does not have an implementation.",
		);

		// Deserialize and set the context.
		for (let [key, value] of Object.entries(serializedContext)) {
			context.set(key, await deserializeValue(value));
		}

		// Deserialize the args.
		let args = (await Promise.all(serializedArgs.map(deserializeValue))) as A;

		// Call the implementation.
		let output = await this.implementation(...args);

		// Serialize the output.
		let serializedOutput = await serializeValue(output);

		return serializedOutput;
	}
}
