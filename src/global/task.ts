import { Artifact } from "./artifact.ts";
import { Block } from "./block.ts";
import { Checksum } from "./checksum.ts";
import { Id } from "./id.ts";
import { placeholder } from "./placeholder.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { System } from "./system.ts";
import { Template, template } from "./template.ts";

export let task = async (arg: Unresolved<Task.Arg>): Promise<Task> => {
	return await Task.new(arg);
};

export let run = async (
	arg: Unresolved<Task.Arg>,
): Promise<Artifact | undefined> => {
	let task = await Task.new(arg);
	let output = await task.run();
	return output;
};

export let output = placeholder("output");

type ConstructorArg = {
	block: Block;
	host: System;
	executable: Template;
	env: Record<string, Template>;
	args: Array<Template>;
	checksum?: Checksum;
	unsafe: boolean;
	network: boolean;
};

export class Task {
	#block: Block;
	#host: System;
	#executable: Template;
	#env: Record<string, Template>;
	#args: Array<Template>;
	#checksum: Checksum | undefined;
	#unsafe: boolean;
	#network: boolean;

	constructor(arg: ConstructorArg) {
		this.#block = arg.block;
		this.#host = arg.host;
		this.#executable = arg.executable;
		this.#env = arg.env;
		this.#args = arg.args;
		this.#checksum = arg.checksum;
		this.#unsafe = arg.unsafe;
		this.#network = arg.network;
	}

	static async new(arg: Unresolved<Task.Arg>): Promise<Task> {
		let resolvedArg = await resolve(arg);
		let host = resolvedArg.host;
		let executable = await template(resolvedArg.executable);
		let env: Record<string, Template> = Object.fromEntries(
			await Promise.all(
				Object.entries(resolvedArg.env ?? {}).map(async ([key, value]) => [
					key,
					await template(value),
				]),
			),
		);
		let args = await Promise.all(
			(resolvedArg.args ?? []).map(async (arg) => await template(arg)),
		);
		let checksum = resolvedArg.checksum ?? undefined;
		let unsafe = resolvedArg.unsafe ?? false;
		let network = resolvedArg.network ?? false;

		return await syscall.task.new({
			host,
			executable,
			env,
			args,
			checksum,
			unsafe,
			network,
		});
	}

	id(): Id {
		return this.block().id();
	}

	block(): Block {
		return this.#block;
	}

	host(): System {
		return this.#host;
	}

	executable(): Template {
		return this.#executable;
	}

	env(): Record<string, Template> {
		return this.#env;
	}

	args(): Array<Template> {
		return this.#args;
	}

	checksum(): Checksum | undefined {
		return this.#checksum;
	}

	unsafe(): boolean {
		return this.#unsafe;
	}

	network(): boolean {
		return this.#network;
	}

	async run(): Promise<Artifact | undefined> {
		return (await syscall.operation.evaluate(this)) as Artifact | undefined;
	}
}

export namespace Task {
	export type Arg = {
		host: System;
		executable: Template.Arg;
		env?: Record<string, Template.Arg>;
		args?: Array<Template.Arg>;
		checksum?: Checksum;
		unsafe?: boolean;
		network?: boolean;
	};
}
