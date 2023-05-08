import { Artifact } from "./artifact.ts";
import { Checksum } from "./checksum.ts";
import { Operation } from "./operation.ts";
import { placeholder } from "./placeholder.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { System } from "./system.ts";
import { Template, template } from "./template.ts";
import { Value } from "./value.ts";

export let process = async (
	arg: Unresolved<Process.Arg>,
): Promise<Artifact> => {
	// Create the process.
	let process = await Process.new(arg);

	// Run the process.
	let output = await process.run();

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

export class Process {
	#hash: Operation.Hash;
	#system: System;
	#executable: Template;
	#env: Record<string, Template>;
	#args: Array<Template>;
	#checksum?: Checksum;
	#unsafe: boolean;
	#network: boolean;
	#hostPaths: Array<string>;

	static async new(arg: Unresolved<Process.Arg>): Promise<Process> {
		// Resolve the args.
		let resolvedArg = await resolve(arg);

		// Create the process.
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
		return Process.fromSyscall(
			await syscall.process.new({
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

	hash(): Operation.Hash {
		return this.#hash;
	}

	toSyscall(): syscall.Process {
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

	static fromSyscall(process: syscall.Process): Process {
		let hash = process.hash;
		let system = process.system;
		let executable = Template.fromSyscall(process.executable);
		let env = Object.fromEntries(
			Object.entries(process.env).map(([key, value]) => [
				key,
				Template.fromSyscall(value),
			]),
		);
		let args = process.args.map((arg) => Template.fromSyscall(arg));
		let checksum = process.checksum;
		let unsafe = process.unsafe;
		let network = process.network;
		let hostPaths = process.hostPaths;
		return new Process({
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

	async run(): Promise<Artifact> {
		let outputFromSyscall = await syscall.operation.run(
			Operation.toSyscall(this),
		);
		let output = Value.fromSyscall(outputFromSyscall);
		return output as Artifact;
	}
}

export namespace Process {
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
