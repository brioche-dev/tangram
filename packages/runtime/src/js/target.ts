import { assert as assert_, unreachable } from "./assert.ts";
import { Checksum } from "./checksum.ts";
import * as encoding from "./encoding.ts";
import { Module } from "./module.ts";
import { Object_ } from "./object.ts";
import { Package } from "./package.ts";
import { MaybePromise, Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { System } from "./system.ts";
import { Template, template } from "./template.ts";
import { MaybeNestedArray, flatten } from "./util.ts";
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
>(...args: Array<Unresolved<Target.Arg>>): Promise<Target<A, R>>;
export function target<
	A extends Array<Value> = Array<Value>,
	R extends Value = Value,
>(
	...args: [FunctionArg<A, R>] | Array<Unresolved<Target.Arg>>
): MaybePromise<Target<A, R>> {
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
	>(...args: Array<Unresolved<Target.Arg>>): Promise<Target<A, R>> {
		let { host, executable, package_, name, env, args_, checksum, unsafe_ } =
			flatten(
				await Promise.all(
					args.map(async function map(
						unresolvedArg: Unresolved<Target.Arg>,
					): Promise<
						MaybeNestedArray<{
							host?: System;
							executable?: Template;
							package_?: Package;
							name?: string;
							env?: Record<string, Value>;
							args_?: Array<Value>;
							checksum?: Checksum;
							unsafe_?: boolean;
						}>
					> {
						let arg = await resolve(unresolvedArg);
						if (Template.Arg.is(arg)) {
							return {
								host: (await current.env())["TANGRAM_HOST"] as System,
								executable: await template("/bin/sh"),
								args_: ["-c", await template(arg)],
							};
						} else if (Target.is(arg)) {
							return {
								host: await arg.host(),
								executable: await arg.executable(),
								package_: await arg.package(),
								name: await arg.name_(),
								env: await arg.env(),
								args_: await arg.args(),
								checksum: await arg.checksum(),
								unsafe_: await arg.unsafe(),
							};
						} else if (arg instanceof Array) {
							return await Promise.all(arg.map(map));
						} else if (typeof arg === "object") {
							return {
								host: arg.host,
								executable: arg.executable
									? await template(arg.executable)
									: undefined,
								package_: arg.package,
								name: arg.name,
								env: arg.env,
								args_: arg.args,
								checksum: arg.checksum,
								unsafe_: arg.unsafe,
							};
						} else {
							return unreachable();
						}
					}),
				),
			).reduce<{
				host?: System;
				executable?: Template;
				package_?: Package;
				name?: string;
				env?: Record<string, Value>;
				args_?: Array<Value>;
				checksum?: Checksum;
				unsafe_?: boolean;
			}>((a, b) => {
				return {
					host: a.host ?? b.host,
					executable: a.executable ?? b.executable,
					package_: a.package_ ?? b.package_,
					name: a.name ?? b.name,
					env: { ...(a.env ?? {}), ...(b.env ?? {}) },
					args_: [...(a.args_ ?? []), ...(b.args_ ?? [])],
					checksum: a.checksum ?? b.checksum,
					unsafe_: a.unsafe_ ?? b.unsafe_,
				};
			}, {});
		if (!host) {
			throw new Error("Cannot create a target without a host.");
		}
		if (!executable) {
			throw new Error("Cannot create a target without an executable.");
		}
		env ??= {};
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
	export type Arg = Template.Arg | Target | Array<Arg> | ArgObject;

	export type ArgObject = {
		host?: System;
		executable?: Template.Arg;
		package?: Package;
		name?: string;
		env?: Record<string, Value>;
		args?: Array<Value>;
		checksum?: Checksum;
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
