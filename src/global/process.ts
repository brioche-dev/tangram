import "./syscall";
import { Artifact } from "./artifact";
import { run } from "./operation";
import { placeholder } from "./placeholder";
import { Unresolved, resolve } from "./resolve";
import { System } from "./system";
import { Template, TemplateLike } from "./template";

type ProcessArgs = {
	system: System;
	env?: Record<string, TemplateLike> | null | undefined;
	command: TemplateLike;
	args?: Array<TemplateLike> | null | undefined;
	unsafe?: boolean | null | undefined;
};

export let process = async (
	args: Unresolved<ProcessArgs>,
): Promise<Artifact> => {
	let resolvedArgs = await resolve(args);
	let system = resolvedArgs.system;
	let env = resolvedArgs.env
		? Object.fromEntries(
				Object.entries(resolvedArgs.env).map(([key, value]) => [
					key,
					new Template(value),
				]),
		  )
		: resolvedArgs.env;
	let command = new Template(resolvedArgs.command);
	let args_ = resolvedArgs.args
		? resolvedArgs.args.map((arg) => new Template(arg))
		: resolvedArgs.args;
	let unsafe = resolvedArgs.unsafe;
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
	workingDirectory?: Template | null | undefined;
	env?: Record<string, Template> | null | undefined;
	command: Template;
	args?: Array<Template> | null | undefined;
	unsafe?: boolean | null | undefined;
};

export class Process {
	system: System;
	env: Record<string, Template> | null | undefined;
	command: Template;
	args: Array<Template> | null | undefined;
	unsafe: boolean | null | undefined;

	constructor(args: ProcessConstructorArgs) {
		this.system = args.system;
		this.env = args.env;
		this.command = args.command;
		this.args = args.args;
		this.unsafe = args.unsafe;
	}

	static isProcess(value: unknown): value is Process {
		return value instanceof Process;
	}

	async serialize(): Promise<syscall.Process> {
		let system = this.system;
		let env = this.env
			? Object.fromEntries(
					await Promise.all(
						Object.entries(this.env).map(async ([key, value]) => [
							key,
							await value.serialize(),
						]),
					),
			  )
			: null;
		let command = await this.command.serialize();
		let args = this.args
			? await Promise.all(this.args.map((arg) => arg.serialize()))
			: null;
		let unsafe = this.unsafe;
		return {
			system,
			env,
			command,
			args,
			unsafe,
		};
	}

	static async deserialize(process: syscall.Process): Promise<Process> {
		let system = process.system;
		let env = process.env
			? Object.fromEntries(
					await Promise.all(
						Object.entries(process.env).map(async ([key, value]) => [
							key,
							await Template.deserialize(value),
						]),
					),
			  )
			: null;
		let command = await Template.deserialize(process.command);
		let args = process.args
			? await Promise.all(process.args.map((arg) => Template.deserialize(arg)))
			: null;
		let unsafe = process.unsafe;
		return new Process({
			system,
			env,
			command,
			args,
			unsafe,
		});
	}

	async run(): Promise<Artifact> {
		return await run(this);
	}
}
