import { Artifact } from "./artifact";
import { run } from "./operation";
import { placeholder } from "./placeholder";
import { Unresolved, resolve } from "./resolve";
import * as syscall from "./syscall";
import { Template, TemplateLike, template } from "./template";
import { nullish } from "./value";

type ProcessArgs = {
	system: System;
	command: TemplateLike;
	env?: Record<string, TemplateLike> | nullish;
	args?: Array<TemplateLike> | nullish;
	unsafe?: boolean | nullish;
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
	let unsafe = resolvedArgs.unsafe ?? false;
	return await new Process({
		system,
		env,
		command,
		args: args_,
		unsafe,
	}).run();
};

export let output = placeholder("output");

export type ProcessConstructorArgs = {
	system: System;
	command: Template;
	env: Record<string, Template>;
	args: Array<Template>;
	unsafe: boolean;
};

export class Process {
	#system: System;
	#command: Template;
	#env: Record<string, Template>;
	#args: Array<Template>;
	#unsafe: boolean;

	constructor(args: ProcessConstructorArgs) {
		this.#system = args.system;
		this.#command = args.command;
		this.#env = args.env;
		this.#args = args.args;
		this.#unsafe = args.unsafe;
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
		let unsafe = this.#unsafe;
		return {
			system,
			command,
			env,
			args,
			unsafe,
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
		let unsafe = process.unsafe;
		return new Process({
			system,
			command,
			env,
			args,
			unsafe,
		});
	}

	async run(): Promise<Artifact> {
		return await run(this);
	}
}
