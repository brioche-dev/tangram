import { assert as assert_ } from "./assert.ts";
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
		let env = resolvedArg.env ?? {};
		let args = resolvedArg.args ?? [];
		let checksum = resolvedArg.checksum ?? undefined;
		let unsafe = resolvedArg.unsafe ?? false;
		let network = resolvedArg.network ?? false;
		return new Task(
			Object_.Handle.withObject({
				kind: "task",
				value: {
					package: package_,
					host,
					executable,
					target,
					env,
					args,
					checksum,
					unsafe,
					network,
				},
			}),
		);
	}

	async id(): Promise<Task.Id> {
		return (await this.#handle.id()) as Package.Id;
	}

	async object(): Promise<Task.Object_> {
		let object = await this.#handle.object();
		assert_(object.kind === "task");
		return object.value;
	}

	handle(): Object_.Handle {
		return this.#handle;
	}

	async host(): Promise<System> {
		return (await this.object()).host;
	}

	async executable(): Promise<Template> {
		return (await this.object()).executable;
	}

	async package(): Promise<Package | undefined> {
		return (await this.object()).package;
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
		host: System;
		executable: Template.Arg;
		package?: Package | undefined;
		target?: string | undefined;
		env?: Record<string, Value>;
		args?: Array<Value>;
		checksum?: Checksum;
		unsafe?: boolean;
		network?: boolean;
	};

	export type Id = string;

	export type Object_ = {
		host: System;
		executable: Template;
		package: Package | undefined;
		target: string | undefined;
		env: Record<string, Value>;
		args: Array<Value>;
		checksum: Checksum | undefined;
		unsafe: boolean;
		network: boolean;
	};
}
