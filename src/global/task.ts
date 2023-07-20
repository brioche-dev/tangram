import { Artifact } from "./artifact.ts";
import { Block } from "./block.ts";
import { Checksum } from "./checksum.ts";
import { Operation } from "./operation.ts";
import { placeholder } from "./placeholder.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { System } from "./system.ts";
import { Template, template } from "./template.ts";
import { Value } from "./value.ts";

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
	system: System;
	executable: Template;
	env: Record<string, Template>;
	args: Array<Template>;
	checksum?: Checksum;
	unsafe: boolean;
	network: boolean;
};

export class Task {
	#block: Block;
	#system: System;
	#executable: Template;
	#env: Record<string, Template>;
	#args: Array<Template>;
	#checksum?: Checksum;
	#unsafe: boolean;
	#network: boolean;

	static async new(arg: Unresolved<Task.Arg>): Promise<Task> {
		let resolvedArg = await resolve(arg);
		let system = resolvedArg.system;
		let executable = await template(resolvedArg.executable);
		let env: Record<string, Template> = Object.fromEntries(
			await Promise.all(
				Object.entries(resolvedArg.env ?? {}).map(async ([key, value]) => [
					key,
					await template(value),
				]),
			),
		);
		let env_ = Object.fromEntries(
			Object.entries(env).map(([key, value]) => [key, value.toSyscall()]),
		);
		let args_ = await Promise.all(
			(resolvedArg.args ?? []).map(async (arg) =>
				(await template(arg)).toSyscall(),
			),
		);
		let checksum = resolvedArg.checksum ?? undefined;
		let unsafe = resolvedArg.unsafe ?? false;
		let network = resolvedArg.network ?? false;
		return Task.fromSyscall(
			syscall.task.new({
				system,
				executable: executable.toSyscall(),
				env: env_,
				args: args_,
				checksum,
				unsafe,
				network,
			}),
		);
	}

	constructor(arg: ConstructorArg) {
		this.#block = arg.block;
		this.#system = arg.system;
		this.#executable = arg.executable;
		this.#env = arg.env;
		this.#args = arg.args;
		this.#checksum = arg.checksum;
		this.#unsafe = arg.unsafe;
		this.#network = arg.network;
	}

	toSyscall(): syscall.Task {
		let block = this.#block.toSyscall();
		let system = this.#system;
		let executable = this.#executable.toSyscall();
		let env = Object.fromEntries(
			Object.entries(this.#env).map(([key, value]) => [key, value.toSyscall()]),
		);
		let args = this.#args.map((arg) => arg.toSyscall());
		let checksum = this.#checksum;
		let unsafe = this.#unsafe;
		let network = this.#network;
		return {
			block,
			system,
			executable,
			env,
			args,
			checksum,
			unsafe,
			network,
		};
	}

	static fromSyscall(task: syscall.Task): Task {
		let block = Block.fromSyscall(task.block);
		let system = task.system;
		let executable = Template.fromSyscall(task.executable);
		let env = Object.fromEntries(
			Object.entries(task.env).map(([key, value]) => [
				key,
				Template.fromSyscall(value),
			]),
		);
		let args = task.args.map((arg) => Template.fromSyscall(arg));
		let checksum = task.checksum;
		let unsafe = task.unsafe;
		let network = task.network;
		return new Task({
			block,
			system,
			executable,
			env,
			args,
			checksum,
			unsafe,
			network,
		});
	}

	block(): Block {
		return this.#block;
	}

	async run(): Promise<Artifact | undefined> {
		let outputFromSyscall = await syscall.operation.evaluation(
			Operation.toSyscall(this),
		);
		let output = Value.fromSyscall(outputFromSyscall);
		return output as Artifact;
	}
}

export namespace Task {
	export type Arg = {
		system: System;
		executable: Template.Arg;
		env?: Record<string, Template.Arg>;
		args?: Array<Template.Arg>;
		checksum?: Checksum;
		unsafe?: boolean;
		network?: boolean;
	};
}
