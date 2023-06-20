import { Artifact } from "./artifact.ts";
import { Checksum } from "./checksum.ts";
import { Operation } from "./operation.ts";
import { placeholder } from "./placeholder.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { System } from "./system.ts";
import { Template, template } from "./template.ts";
import { Value } from "./value.ts";

export let command = async (arg: Unresolved<Command.Arg>): Promise<Command> => {
	return await Command.new(arg);
};

export let run = async (
	arg: Unresolved<Command.Arg>,
): Promise<Artifact | undefined> => {
	let command = await Command.new(arg);
	let output = await command.run();
	return output;
};

export let output = placeholder("output");

type ConstructorArg = {
	hash: Operation.Hash;
	system: System;
	executable: Template;
	env: Record<string, Template>;
	args: Array<Template>;
	checksum?: Checksum;
	unsafe: boolean;
	network: boolean;
};

export class Command {
	#hash: Operation.Hash;
	#system: System;
	#executable: Template;
	#env: Record<string, Template>;
	#args: Array<Template>;
	#checksum?: Checksum;
	#unsafe: boolean;
	#network: boolean;

	static async new(arg: Unresolved<Command.Arg>): Promise<Command> {
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
		return Command.fromSyscall(
			syscall.command.new({
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
		this.#hash = arg.hash;
		this.#system = arg.system;
		this.#executable = arg.executable;
		this.#env = arg.env;
		this.#args = arg.args;
		this.#checksum = arg.checksum;
		this.#unsafe = arg.unsafe;
		this.#network = arg.network;
	}

	toSyscall(): syscall.Command {
		let hash = this.#hash;
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
			hash,
			system,
			executable,
			env,
			args,
			checksum,
			unsafe,
			network,
		};
	}

	static fromSyscall(command: syscall.Command): Command {
		let hash = command.hash;
		let system = command.system;
		let executable = Template.fromSyscall(command.executable);
		let env = Object.fromEntries(
			Object.entries(command.env).map(([key, value]) => [
				key,
				Template.fromSyscall(value),
			]),
		);
		let args = command.args.map((arg) => Template.fromSyscall(arg));
		let checksum = command.checksum;
		let unsafe = command.unsafe;
		let network = command.network;
		return new Command({
			hash,
			system,
			executable,
			env,
			args,
			checksum,
			unsafe,
			network,
		});
	}

	hash(): Operation.Hash {
		return this.#hash;
	}

	async run(): Promise<Artifact | undefined> {
		let outputFromSyscall = await syscall.operation.run(
			Operation.toSyscall(this),
		);
		let output = Value.fromSyscall(outputFromSyscall);
		return output as Artifact;
	}
}

export namespace Command {
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
