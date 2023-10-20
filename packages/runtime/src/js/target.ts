import { Args, flatten } from "./args.ts";
import { assert as assert_, unreachable } from "./assert.ts";
import { Checksum } from "./checksum.ts";
import * as encoding from "./encoding.ts";
import { Module } from "./module.ts";
import { Object_ } from "./object.ts";
import { Package } from "./package.ts";
import { MaybePromise, Unresolved } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { System } from "./system.ts";
import { Template, template } from "./template.ts";
import { Value } from "./value.ts";

export let current: Target;

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
					args: args as Array<Value>,
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
			env?:
				| Record<string, Value>
				| Array<Record<string, EnvMutation>>
				| undefined;
			args: Array<Value>;
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
		} = await Args.apply<Target.Arg, Apply>(args, async (arg) => {
			if (Template.Arg.is(arg)) {
				let host = {
					kind: "set" as const,
					value: (await current.env())["TANGRAM_HOST"] as System,
				};
				let executable = {
					kind: "set" as const,
					value: await template("/bin/sh"),
				};
				let args_ = {
					kind: "set" as const,
					value: ["-c", await template(arg)],
				};
				return { host, executable, args_ };
			} else if (Target.is(arg)) {
				let host = { kind: "set" as const, value: await arg.host() };
				let executable = {
					kind: "set" as const,
					value: await arg.executable(),
				};
				let package_ = { kind: "set" as const, value: await arg.package() };
				let name = { kind: "set" as const, value: await arg.name_() };
				let env = { kind: "set" as const, value: await arg.env() };
				let args_ = { kind: "set" as const, value: await arg.args() };
				let checksum = { kind: "set" as const, value: await arg.checksum() };
				let unsafe = { kind: "set" as const, value: await arg.unsafe() };
				return {
					host,
					executable,
					package: package_,
					name,
					env,
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
					object.env =
						arg.env === undefined
							? { kind: "unset" }
							: {
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
		let env =
			env_ && env_ instanceof Array
				? await processEnvMutations({}, ...(env_ ?? []))
				: env_ ?? {};
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
	export type Arg = Template.Arg | Target | ArgObject;

	export type ArgObject = {
		host?: System;
		executable?: Template.Arg;
		package?: Package | undefined;
		name?: string | undefined;
		env?: Record<string, EnvMutation> | undefined;
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

type EnvMutation =
	| Template.Arg
	| { kind: "unset" }
	| { kind: "set"; value: Template.Arg }
	| { kind: "set_if_unset"; value: Template.Arg }
	| {
			kind: "append";
			value: Args.MaybeNestedArray<Template.Arg>;
			separator?: Template.Arg;
	  }
	| {
			kind: "prepend";
			value: Args.MaybeNestedArray<Template.Arg>;
			separator?: Template.Arg;
	  };

/** Collect a list of env mutations into a single environment. */
let processEnvMutations = async (
	init: Record<string, Value>,
	...args: Array<Record<string, EnvMutation>>
): Promise<Record<string, Value>> => {
	let env = { ...init };
	for (let arg of args) {
		// Apply mutations for a single argument.
		for (let [key, mutation] of Object.entries(arg)) {
			await mutateEnv(env, key, mutation);
		}
	}
	return env;
};

/** Mutate an env object in-place. */
let mutateEnv = async (
	env: Record<string, Value>,
	key: string,
	mutation: EnvMutation,
) => {
	if (Template.Arg.is(mutation)) {
		mutation = { kind: "set", value: mutation };
	}
	if (mutation.kind === "unset") {
		delete env[key];
	} else if (mutation.kind === "set") {
		env[key] = mutation.value;
	} else if (mutation.kind === "set_if_unset") {
		if (!(key in env)) {
			env[key] = mutation.value;
		}
	} else if (mutation.kind === "append") {
		if (!(key in env)) {
			env[key] = await template();
		}
		let t = env[key];
		assert_(Template.Arg.is(t));
		env[key] = await Template.join(
			mutation.separator ?? "",
			t,
			...flatten(mutation.value),
		);
	} else if (mutation.kind === "prepend") {
		if (!(key in env)) {
			env[key] = await template();
		}
		let t = env[key];
		assert_(Template.Arg.is(t));
		env[key] = await Template.join(
			mutation.separator ?? "",
			...flatten(mutation.value),
			t,
		);
	}
};
