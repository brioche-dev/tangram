import { Artifact } from "./artifact.ts";
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

export class Task {
	#id: Id | undefined;
	#data: Task.Data | undefined;

	constructor(arg: Task.Data) {
		this.#data = arg;
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
		return new Task({
			host,
			executable,
			env,
			args,
			checksum,
			unsafe,
			network,
		});
	}

	async load(): Promise<void> {
		if (!this.#data) {
			this.#data = ((await syscall.value.load(this)) as Task).#data;
		}
	}

	async store(): Promise<void> {
		if (!this.#id) {
			this.#id = ((await syscall.value.store(this)) as Task).#id;
		}
	}

	async host(): Promise<System> {
		await this.load();
		return this.#data!.host;
	}

	async executable(): Promise<Template> {
		await this.load();
		return this.#data!.executable;
	}

	async env(): Promise<Record<string, Template>> {
		await this.load();
		return this.#data!.env;
	}

	async args(): Promise<Array<Template>> {
		await this.load();
		return this.#data!.args;
	}

	async checksum(): Promise<Checksum | undefined> {
		await this.load();
		return this.#data!.checksum;
	}

	async unsafe(): Promise<boolean> {
		await this.load();
		return this.#data!.unsafe;
	}

	async network(): Promise<boolean> {
		await this.load();
		return this.#data!.network;
	}

	async run(): Promise<Artifact | undefined> {
		return (await syscall.build.output(this)) as Artifact | undefined;
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

	export type Data = {
		host: System;
		executable: Template;
		env: Record<string, Template>;
		args: Array<Template>;
		checksum: Checksum | undefined;
		unsafe: boolean;
		network: boolean;
	};
}
