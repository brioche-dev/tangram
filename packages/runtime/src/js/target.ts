import { Artifact } from "./artifact.ts";
import { assert as assert_, unreachable } from "./assert.ts";
import { Checksum } from "./checksum.ts";
import * as encoding from "./encoding.ts";
import { Module } from "./module.ts";
import { Args, MaybePromise, apply as apply_ } from "./mutation.ts";
import { Object_ } from "./object.ts";
import { Package } from "./package.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { System } from "./system.ts";
import { Template, template } from "./template.ts";
import { Value } from "./value.ts";

let current: Target;
export let getCurrent = (): Target => {
	return current;
};
export let setCurrent = (target: Target) => {
	current = target;
};

export let functions: Record<string, Function> = {};

type FunctionArg<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
> = {
	url: string;
	name: string;
	function: (...args: A) => MaybePromise<R | void>;
};

export function target<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
>(args: FunctionArg): Target<A, R>;
export function target<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
>(...args: Args<Target.Arg>): Promise<Target<A, R>>;
export function target<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
>(...args: [FunctionArg<A, R>] | Args<Target.Arg>): MaybePromise<Target<A, R>> {
	if (
		args.length === 1 &&
		typeof args[0] === "object" &&
		"function" in args[0]
	) {
		// Register the function.
		let arg = args[0];
		let { url, name } = arg;
		let key = encoding.json.encode({ url, name });
		assert_(functions[key] === undefined);
		functions[key] = arg.function;

		// Get the package.
		let module_ = Module.fromUrl(arg.url);
		assert_(module_.kind === "normal");
		let package_ = Package.withId(module_.value.packageId);

		// Create the target.
		return new Target(
			Object_.Handle.withObject({
				kind: "target",
				value: {
					host: "js-js",
					executable: new Template([module_.value.path]),
					package: package_,
					name: arg.name,
					args: [],
					env: {},
					checksum: undefined,
					unsafe: false,
				},
			}),
		);
	} else {
		return Target.new(...args);
	}
}

export let build = async (
	...args: Array<Unresolved<Target.Arg>>
): Promise<Value> => {
	return await (await target(...args)).build();
};

export interface Target<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
> extends globalThis.Function {
	(...args: { [K in keyof A]: Unresolved<A[K]> }): Promise<R>;
}

export class Target<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
> extends globalThis.Function {
	#handle: Object_.Handle;

	constructor(handle: Object_.Handle) {
		super();
		this.#handle = handle;
		let this_ = this as any;
		return new Proxy(this_, {
			get(_target, prop, _receiver) {
				if (typeof this_[prop] === "function") {
					return this_[prop].bind(this_);
				} else {
					return this_[prop];
				}
			},
			apply: async (_target, _, args) => {
				let target = await Target.new(this_ as any, {
					args: (await resolve(args)) as Array<Value>,
				});
				return await target.build();
			},
			getPrototypeOf: (_target) => {
				return Object.getPrototypeOf(this_);
			},
		});
	}

	static withId(id: Target.Id): Target {
		return new Target(Object_.Handle.withId(id));
	}

	static async new<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	>(...args: Args<Target.Arg>): Promise<Target<A, R>> {
		type Apply = {
			host?: System;
			executable?: Template;
			package?: Package | undefined;
			name?: string | undefined;
			env?: Array<Args.MutationObject>;
			args?: Array<Value>;
			checksum?: Checksum | undefined;
			unsafe?: boolean;
		};
		let {
			host,
			executable,
			package: package_,
			name,
			env: env_,
			args: args_,
			checksum,
			unsafe: unsafe_,
		} = await apply_<Target.Arg, Apply>(args, async (arg) => {
			if (
				typeof arg === "string" ||
				Artifact.is(arg) ||
				arg instanceof Template
			) {
				let host = {
					kind: "set" as const,
					value: (await getCurrent().env())["TANGRAM_HOST"] as System,
				};
				let executable = {
					kind: "set" as const,
					value: await template("/bin/sh"),
				};
				let args = {
					kind: "set" as const,
					value: ["-c", await template(arg)],
				};
				return { host, executable, args };
			} else if (Target.is(arg)) {
				let host = { kind: "set" as const, value: await arg.host() };
				let executable = {
					kind: "set" as const,
					value: await arg.executable(),
				};
				let package_ = { kind: "set" as const, value: await arg.package() };
				let name = { kind: "set" as const, value: await arg.name_() };
				let env_ = {
					kind: "set" as const,
					value: [
						Object.fromEntries(
							Object.entries(await arg.env()).map(([key, value]) => [
								key,
								{ kind: "set" as const, value },
							]),
						),
					],
				};
				let args_ = { kind: "set" as const, value: await arg.args() };
				let checksum = { kind: "set" as const, value: await arg.checksum() };
				let unsafe = { kind: "set" as const, value: await arg.unsafe() };
				return {
					host,
					executable,
					package: package_,
					name,
					env: env_,
					args: args_,
					checksum,
					unsafe,
				};
			} else if (typeof arg === "object") {
				let object: Args.MutationObject<Apply> = {};
				if ("host" in arg) {
					object.host = {
						kind: "set" as const,
						value: arg.host,
					};
				}
				if ("executable" in arg) {
					object.executable = {
						kind: "set" as const,
						value: await template(arg.executable),
					};
				}
				if ("package" in arg) {
					object.package = {
						kind: "set" as const,
						value: arg.package,
					};
				}
				if ("name" in arg) {
					object.name = {
						kind: "set" as const,
						value: arg.name,
					};
				}
				if ("env" in arg) {
					object.env = {
						kind: "append" as const,
						value: arg.env,
					};
				}
				if ("args" in arg) {
					object.args = {
						kind: "append" as const,
						value: arg.args,
					};
				}
				if ("checksum" in arg) {
					object.checksum = {
						kind: "set" as const,
						value: arg.checksum,
					};
				}
				if ("unsafe" in arg) {
					object.unsafe = {
						kind: "set" as const,
						value: arg.unsafe,
					};
				}
				return object;
			} else {
				return unreachable();
			}
		});
		if (!host) {
			throw new Error("Cannot create a target without a host.");
		}
		if (!executable) {
			throw new Error("Cannot create a target without an executable.");
		}
		let env = await Args.apply<Args.MutationObject, { [key: string]: Value }>(
			env_ ?? [],
			async (arg) => arg,
		);
		args_ ??= [];
		unsafe_ ??= false;
		return new Target(
			Object_.Handle.withObject({
				kind: "target",
				value: {
					host,
					executable,
					package: package_,
					name,
					env,
					args: args_,
					checksum,
					unsafe: unsafe_,
				},
			}),
		);
	}

	static is(value: unknown): value is Target {
		return value instanceof Target;
	}

	static expect(value: unknown): Target {
		assert_(Target.is(value));
		return value;
	}

	static assert(value: unknown): asserts value is Target {
		assert_(Target.is(value));
	}

	async id(): Promise<Target.Id> {
		return (await this.#handle.id()) as Target.Id;
	}

	async object(): Promise<Target.Object_> {
		let object = await this.#handle.object();
		assert_(object.kind === "target");
		return object.value;
	}

	get handle(): Object_.Handle {
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

	async name_(): Promise<string | undefined> {
		return (await this.object()).name;
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

	async build(...args: A): Promise<Value> {
		return await syscall.build(
			await Target.new<[], R>(this as Target, { args }),
		);
	}
}

export namespace Target {
	export type Arg =
		| undefined
		| string
		| Artifact
		| Template
		| Target
		| ArgObject
		| Array<Arg>;

	export type ArgObject = {
		host?: System;
		executable?: Template.Arg;
		package?: Package | undefined;
		name?: string | undefined;
		env?: Record<string, Args.Mutation<Value>>;
		args?: Array<Value>;
		checksum?: Checksum | undefined;
		unsafe?: boolean;
	};

	export type Id = string;

	export type Object_ = {
		host: System;
		executable: Template;
		package: Package | undefined;
		name: string | undefined;
		env: Record<string, Value>;
		args: Array<Value>;
		checksum: Checksum | undefined;
		unsafe: boolean;
	};
}
