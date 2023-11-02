import { Artifact } from "./artifact.ts";
import { assert as assert_, unreachable } from "./assert.ts";
import { Checksum } from "./checksum.ts";
import { Directory } from "./directory.ts";
import * as encoding from "./encoding.ts";
import { Lock } from "./lock.ts";
import { Module } from "./module.ts";
import {
	Args,
	MaybeNestedArray,
	MutationMap,
	apply,
	flatten,
	mutation,
} from "./mutation.ts";
import { Object_ } from "./object.ts";
import { MaybePromise, Unresolved } from "./resolve.ts";
import { Symlink } from "./symlink.ts";
import * as syscall from "./syscall.ts";
import { System } from "./system.ts";
import { Template } from "./template.ts";
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
	function: (...args: A) => Unresolved<R>;
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
		let lock = Lock.withId(module_.value.lock);

		// Create the executable.
		let executable = new Symlink({
			object: {
				target: new Template([
					Directory.withId(module_.value.package),
					"/" + module_.value.path,
				]),
			},
		});

		// Create the target.
		return new Target({
			object: {
				host: "js-js",
				executable,
				lock,
				name: arg.name,
				args: [],
				env: {},
				checksum: undefined,
				unsafe: false,
			},
		});
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
	#state: Target.State;

	constructor(state: Target.State) {
		super();
		this.#state = state;
		let this_ = this as any;
		return new Proxy(this_, {
			get(_target, prop, _receiver) {
				if (typeof this_[prop] === "function") {
					return this_[prop].bind(this_);
				} else {
					return this_[prop];
				}
			},
			apply: async (target, _, args) => {
				return await target.build(...args);
			},
			getPrototypeOf: (_target) => {
				return Object.getPrototypeOf(this_);
			},
		});
	}

	get state(): Target.State {
		return this.#state;
	}

	static withId(id: Target.Id): Target {
		return new Target({ id });
	}

	static async new<
		A extends Array<Value> = Array<Value>,
		R extends Value = Value,
	>(...args: Args<Target.Arg>): Promise<Target<A, R>> {
		type Apply = {
			host?: System;
			executable?: Artifact;
			lock?: Lock | undefined;
			name?: string | undefined;
			env?: MaybeNestedArray<MutationMap>;
			args?: Array<Value>;
			checksum?: Checksum | undefined;
			unsafe?: boolean;
		};
		let {
			host,
			executable,
			lock,
			name,
			env: env_,
			args: args_,
			checksum,
			unsafe: unsafe_,
		} = await apply<Target.Arg, Apply>(args, async (arg) => {
			if (
				typeof arg === "string" ||
				Artifact.is(arg) ||
				arg instanceof Template
			) {
				return {
					host: (await getCurrent().env())["TANGRAM_HOST"] as System,
					executable: new Symlink({
						object: { target: new Template(["/bin/sh"]) },
					}),
					args: ["-c", arg],
				};
			} else if (Target.is(arg)) {
				return await arg.object();
			} else if (typeof arg === "object") {
				let object: MutationMap<Apply> = {};
				if ("env" in arg) {
					object.env =
						arg.env !== undefined
							? await mutation({ kind: "array_append", values: [arg.env] })
							: await mutation({ kind: "unset" });
				}
				if ("args" in arg) {
					object.args =
						arg.args !== undefined
							? await mutation({
									kind: "array_append",
									values: [...arg.args],
							  })
							: await mutation({ kind: "unset" });
				}
				return {
					...arg,
					...object,
				};
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
		let env = await apply(flatten(env_ ?? []), async (arg) => arg);
		args_ ??= [];
		unsafe_ ??= false;
		return new Target({
			object: {
				host,
				executable,
				lock,
				name,
				env,
				args: args_,
				checksum,
				unsafe: unsafe_,
			},
		});
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
		await this.store();
		return this.#state.id!;
	}

	async object(): Promise<Target.Object_> {
		await this.load();
		return this.#state.object!;
	}

	async load() {
		if (this.#state.object === undefined) {
			let object = await syscall.load(this.#state.id!);
			assert_(object.kind === "target");
			this.#state.object = object.value;
		}
	}

	async store() {
		if (this.#state.id === undefined) {
			this.#state.id = await syscall.store({
				kind: "target",
				value: this.#state.object!,
			});
		}
	}

	async host(): Promise<System> {
		return (await this.object()).host;
	}

	async executable(): Promise<Artifact> {
		return (await this.object()).executable;
	}

	async lock(): Promise<Lock | undefined> {
		return (await this.object()).lock;
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
		executable?: Artifact;
		lock?: Lock | undefined;
		name?: string | undefined;
		env?: MutationMap;
		args?: Array<Value>;
		checksum?: Checksum | undefined;
		unsafe?: boolean;
	};

	export type Id = string;

	export type Object_ = {
		host: System;
		executable: Artifact;
		lock: Lock | undefined;
		name: string | undefined;
		env: Record<string, Value>;
		args: Array<Value>;
		checksum: Checksum | undefined;
		unsafe: boolean;
	};

	export type State = Object_.State<Target.Id, Target.Object_>;
}
