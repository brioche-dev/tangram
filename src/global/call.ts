import { assert } from "./assert.ts";
import { Function } from "./function.ts";
import { Operation } from "./operation.ts";
import * as syscall from "./syscall.ts";
import { Value } from "./value.ts";

type Env = {
	value?: Record<string, Value>;
	get(): Record<string, Value>;
};

export let env: Env = {
	get() {
		assert(this.value);
		return this.value;
	},
};

export let call = async <A extends Array<Value>, R extends Value>(
	arg: Call.Arg<A, R>,
): Promise<R> => {
	// Create the call.
	let call = await Call.new(arg);

	// Run the operation.
	let output = await call.run();

	return output;
};

type ConstructorArgs<T extends Value> = {
	hash: Operation.Hash;
	function: Function<any, T>;
	env: Record<string, Value>;
	args: Array<Value>;
};

export class Call<A extends Array<Value> = [], R extends Value = Value> {
	#hash: Operation.Hash;
	#function: Function<A, R>;
	#env: Record<string, Value>;
	#args: Array<Value>;

	static async new<A extends Array<Value>, R extends Value>(
		arg: Call.Arg<A, R>,
	): Promise<Call<A, R>> {
		// Get the function, env, and args.
		let function_ = arg.function.toSyscall();
		let env_ = Object.fromEntries(
			Object.entries(arg.env ?? env.get()).map(([key, value]) => [
				key,
				Value.toSyscall(value),
			]),
		);
		let args_ = (arg.args ?? []).map((arg) => Value.toSyscall(arg));

		// Create the call.
		let call: Call<A, R> = Call.fromSyscall(
			await syscall.call.new({ function: function_, env: env_, args: args_ }),
		);

		return call;
	}

	constructor(args: ConstructorArgs<R>) {
		this.#hash = args.hash;
		this.#function = args.function;
		this.#env = args.env;
		this.#args = args.args;
	}

	static is(value: unknown): value is Call<any, any> {
		return value instanceof Call;
	}

	hash(): Operation.Hash {
		return this.#hash;
	}

	toSyscall(): syscall.Call {
		let hash = this.#hash;
		let function_ = this.#function.toSyscall();
		let env = Object.fromEntries(
			Object.entries(this.#env).map(([key, value]) => [
				key,
				Value.toSyscall(value),
			]),
		);
		let args = this.#args.map((arg) => Value.toSyscall(arg));
		return {
			hash,
			function: function_,
			env,
			args,
		};
	}

	static fromSyscall<A extends Array<Value>, R extends Value>(
		call: syscall.Call,
	): Call<A, R> {
		let hash = call.hash;
		let function_ = Function.fromSyscall<Array<Value>, R>(call.function);
		let env = Object.fromEntries(
			Object.entries(call.env).map(([key, value]) => [
				key,
				Value.fromSyscall(value),
			]),
		);
		let args = call.args.map((arg) => Value.fromSyscall(arg));
		return new Call<A, R>({
			hash,
			function: function_,
			env,
			args,
		});
	}

	async run(): Promise<R> {
		let outputFromSyscall = await syscall.operation.run(
			Operation.toSyscall(this),
		);
		let output = Value.fromSyscall(outputFromSyscall);
		return output as R;
	}
}

export namespace Call {
	export type Arg<A extends Array<Value>, R extends Value> = {
		function: Function<A, R>;
		env?: Record<string, Value>;
		args: A;
	};
}
