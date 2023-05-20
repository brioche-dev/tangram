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

type ConstructorArgs = {
	hash: Operation.Hash;
	system: System;
	executable: Template;
	env: Record<string, Template>;
	args: Array<Template>;
	checksum?: Checksum;
	unsafe: boolean;
	network: boolean;
	hostPaths: Array<string>;
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
	#hostPaths: Array<string>;

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
		let hostPaths = resolvedArg.hostPaths ?? [];
		return Command.fromSyscall(
			await syscall.command.new({
				system,
				executable: executable.toSyscall(),
				env: env_,
				args: args_,
				checksum,
				unsafe,
				network,
				hostPaths,
			}),
		);
	}

	constructor(args: ConstructorArgs) {
		this.#hash = args.hash;
		this.#system = args.system;
		this.#executable = args.executable;
		this.#env = args.env;
		this.#args = args.args;
		this.#checksum = args.checksum;
		this.#unsafe = args.unsafe;
		this.#network = args.network;
		this.#hostPaths = args.hostPaths;
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
		let hostPaths = this.#hostPaths;
		return {
			hash,
			system,
			executable,
			env,
			args,
			checksum,
			unsafe,
			network,
			hostPaths,
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
		let hostPaths = command.hostPaths;
		return new Command({
			hash,
			system,
			executable,
			env,
			args,
			checksum,
			unsafe,
			network,
			hostPaths,
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
		hostPaths?: Array<string>;
	};
}
