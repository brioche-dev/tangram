import { Artifact } from "./artifact.ts";
import { Checksum } from "./checksum.ts";
import { run } from "./operation.ts";
import { placeholder } from "./placeholder.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { Template, TemplateLike, template } from "./template.ts";
import { nullish } from "./value.ts";

type ProcessArgs = {
	system: System;
	command: TemplateLike;
	env?: Record<string, TemplateLike> | nullish;
	args?: Array<TemplateLike> | nullish;
	checksum?: Checksum | nullish;
	unsafe?: boolean | nullish;
	network?: boolean | nullish;
	hostPaths?: Array<string> | nullish;
};

type System = "amd64_linux" | "arm64_linux" | "amd64_macos" | "arm64_macos";

export let process = async (
	args: Unresolved<ProcessArgs>,
): Promise<Artifact> => {
	let resolvedArgs = await resolve(args);
	let system = resolvedArgs.system;
	let command = await template(resolvedArgs.command);
	let env = Object.fromEntries(
		await Promise.all(
			Object.entries(resolvedArgs.env ?? {}).map(async ([key, value]) => [
				key,
				await template(value),
			]),
		),
	);
	let args_ = await Promise.all(
		(resolvedArgs.args ?? []).map(async (arg) => await template(arg)),
	);
	let checksum = resolvedArgs.checksum ?? null;
	let unsafe = resolvedArgs.unsafe ?? false;
	let network = resolvedArgs.network ?? false;
	let hostPaths = resolvedArgs.hostPaths ?? [];
	return await new Process({
		system,
		env,
		command,
		args: args_,
		checksum,
		unsafe,
		network,
		hostPaths,
	}).run();
};

export let output = placeholder("output");

export type ProcessConstructorArgs = {
	system: System;
	command: Template;
	env: Record<string, Template>;
	args: Array<Template>;
	checksum: Checksum | nullish;
	unsafe: boolean;
	network: boolean;
	hostPaths: Array<string>;
};

export class Process {
	#system: System;
	#command: Template;
	#env: Record<string, Template>;
	#args: Array<Template>;
	#checksum: Checksum | nullish;
	#unsafe: boolean;
	#network: boolean;
	#hostPaths: Array<string>;

	constructor(args: ProcessConstructorArgs) {
		this.#system = args.system;
		this.#command = args.command;
		this.#env = args.env;
		this.#args = args.args;
		this.#checksum = args.checksum;
		this.#unsafe = args.unsafe;
		this.#network = args.network;
		this.#hostPaths = args.hostPaths;
	}

	async serialize(): Promise<syscall.Process> {
		let system = this.#system;
		let command = await this.#command.serialize();
		let env = Object.fromEntries(
			await Promise.all(
				Object.entries(this.#env).map(async ([key, value]) => [
					key,
					await value.serialize(),
				]),
			),
		);
		let args = await Promise.all(this.#args.map((arg) => arg.serialize()));
		let checksum = this.#checksum;
		let unsafe = this.#unsafe;
		let network = this.#network;
		let hostPaths = this.#hostPaths;
		return {
			system,
			command,
			env,
			args,
			checksum,
			unsafe,
			network,
			hostPaths,
		};
	}

	static async deserialize(process: syscall.Process): Promise<Process> {
		let system = process.system;
		let command = await Template.deserialize(process.command);
		let env = Object.fromEntries(
			await Promise.all(
				Object.entries(process.env).map(async ([key, value]) => [
					key,
					await Template.deserialize(value),
				]),
			),
		);
		let args = await Promise.all(
			process.args.map((arg) => Template.deserialize(arg)),
		);
		let checksum = process.checksum;
		let unsafe = process.unsafe;
		let network = process.network;
		let hostPaths = process.hostPaths;
		return new Process({
			system,
			command,
			env,
			args,
			checksum,
			unsafe,
			network,
			hostPaths,
		});
	}

	async run(): Promise<Artifact> {
		return await run(this);
	}
}
