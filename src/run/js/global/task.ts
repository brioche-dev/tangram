import { Checksum } from "./checksum.ts";
import { Object_ } from "./object.ts";
import { Package } from "./package.ts";
import { placeholder } from "./placeholder.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { System } from "./system.ts";
import { Template, template } from "./template.ts";
import { Value } from "./value.ts";

export let task = async (arg: Unresolved<Task.Arg>): Promise<Task> => {
	return await Task.new(arg);
};

export let run = async (arg: Unresolved<Task.Arg>): Promise<Value> => {
	let task = await Task.new(arg);
	let output = await task.run();
	return output;
};

export let output = placeholder("output");

export class Task {
	#handle: Object_.Handle;

	constructor(handle: Object_.Handle) {
		this.#handle = handle;
	}

	static async new(arg: Unresolved<Task.Arg>): Promise<Task> {
		let resolvedArg = await resolve(arg);
		let package_ = resolvedArg.package;
		let host = resolvedArg.host;
		let executable = await template(resolvedArg.executable);
		let target = resolvedArg.target;
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
		return new Task(
			Object_.Handle.withObject({
				package: package_,
				host,
				executable,
				target,
				env,
				args,
				checksum,
				unsafe,
				network,
			}),
		);
	}

	async id(): Promise<Task.Id> {
		return (await this.#handle.id()) as Package.Id;
	}

	async object(): Promise<Task.Object> {
		return (await this.#handle.object()) as Task.Object;
	}

	async package(): Promise<Package | undefined> {
		return (await this.object()).package;
	}

	async host(): Promise<System> {
		return (await this.object()).host;
	}

	async executable(): Promise<Template> {
		return (await this.object()).executable;
	}

	async target(): Promise<string | undefined> {
		return (await this.object()).target;
	}

	async env(): Promise<Record<string, Value>> {
		return (await this.object()).env;
	}

	async args(): Promise<Array<Value>> {
		return (await this.object()).args;
	}

	async checksum(): Promise<Checksum | undefined> {
		return (await this.object()).checksum;
	}

	async unsafe(): Promise<boolean> {
		return (await this.object()).unsafe;
	}

	async network(): Promise<boolean> {
		return (await this.object()).network;
	}

	async run(): Promise<Value> {
		return await syscall.task.output(this);
	}
}

export namespace Task {
	export type Arg = {
		package?: Package | undefined;
		host: System;
		executable: Template.Arg;
		target?: string | undefined;
		env?: Record<string, Template.Arg>;
		args?: Array<Template.Arg>;
		checksum?: Checksum;
		unsafe?: boolean;
		network?: boolean;
	};

	export type Id = string;

	export type Object = {
		package: Package | undefined;
		host: System;
		executable: Template;
		target: string | undefined;
		env: Record<string, Template>;
		args: Array<Template>;
		checksum: Checksum | undefined;
		unsafe: boolean;
		network: boolean;
	};
}
