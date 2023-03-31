import { Function } from "./function.ts";
import { run } from "./operation.ts";
import * as syscall from "./syscall.ts";
import { Value, deserializeValue, nullish, serializeValue } from "./value.ts";

type CallArgs<T extends Value> = {
	function: Function<any, T>;
	context?: Map<string, Value> | nullish;
	args?: Array<Value> | nullish;
};

export let call = async <T extends Value = Value>(
	args: CallArgs<T>,
): Promise<T> => {
	let function_ = args.function;
	let context = args.context ?? new Map();
	let args_ = args.args ?? [];
	return await new Call<T>({
		function: function_,
		context,
		args: args_,
	}).run();
};

export let isCall = (value: unknown): value is Call => {
	return value instanceof Call;
};

type CallConstructorArgs<T extends Value> = {
	function: Function<any, T>;
	context: Map<string, Value>;
	args: Array<Value>;
};

export class Call<T extends Value = Value> {
	#function: Function<any, T>;
	#context: Map<string, Value>;
	#args: Array<Value>;

	constructor(args: CallConstructorArgs<T>) {
		this.#function = args.function;
		this.#context = args.context;
		this.#args = args.args;
	}

	async serialize(): Promise<syscall.Call> {
		let function_ = await this.#function.serialize();
		let context = Object.fromEntries(
			await Promise.all(
				Array.from(this.#context.entries()).map(async ([key, value]) => [
					key,
					await serializeValue(value),
				]),
			),
		);
		let args = await Promise.all(this.#args.map((arg) => serializeValue(arg)));
		return {
			function: function_,
			context,
			args,
		};
	}

	static async deserialize<T extends Value>(
		call: syscall.Call,
	): Promise<Call<T>> {
		let function_ = await Function.deserialize<Array<Value>, T>(call.function);
		let context = new Map(
			await Promise.all(
				Object.entries(call.context).map(
					async ([key, value]): Promise<[string, Value]> => [
						key,
						await deserializeValue(value),
					],
				),
			),
		);
		let args = await Promise.all(call.args.map((arg) => deserializeValue(arg)));
		return new Call<T>({
			function: function_,
			context,
			args,
		});
	}

	async run(): Promise<T> {
		return (await run(this)) as T;
	}
}
