import { Artifact } from "./artifact.ts";
import { assert as assert_, unreachable } from "./assert.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Args, MutationMap, apply, mutation } from "./mutation.ts";
import { Object_ } from "./object.ts";
import { Relpath, relpath } from "./path.ts";
import { Unresolved } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { Template, template } from "./template.ts";

export let symlink = async (...args: Args<Symlink.Arg>): Promise<Symlink> => {
	return await Symlink.new(...args);
};

export class Symlink {
	#state: Symlink.State;

	constructor(state: Symlink.State) {
		this.#state = state;
	}

	get state(): Symlink.State {
		return this.#state;
	}

	static withId(id: Symlink.Id): Symlink {
		return new Symlink({ id });
	}

	static async new(...args: Args<Symlink.Arg>): Promise<Symlink> {
		type Apply = {
			artifact?: Artifact | undefined;
			path?: Array<string>;
		};
		let { artifact, path: path_ } = await apply<Symlink.Arg, Apply>(
			args,
			async (arg) => {
				if (arg === undefined) {
					return {};
				} else if (typeof arg === "string") {
					return {
						path: await mutation({ kind: "array_append", values: [arg] }),
					};
				} else if (Artifact.is(arg)) {
					return {
						artifact: arg,
						path: await mutation({ kind: "unset" }),
					};
				} else if (Template.is(arg)) {
					assert_(arg.components.length <= 2);
					let [firstComponent, secondComponent] = arg.components;
					if (
						typeof firstComponent === "string" &&
						secondComponent === undefined
					) {
						return {
							path: await mutation({
								kind: "array_append",
								values: [firstComponent],
							}),
						};
					} else if (
						Artifact.is(firstComponent) &&
						secondComponent === undefined
					) {
						return {
							artifact: firstComponent,
							path: await mutation({ kind: "unset" }),
						};
					} else if (
						Artifact.is(firstComponent) &&
						typeof secondComponent === "string"
					) {
						assert_(secondComponent.startsWith("/"));
						return {
							artifact: firstComponent,
							path: [secondComponent.slice(1)],
						};
					} else {
						throw new Error("Invalid template.");
					}
				} else if (Symlink.is(arg)) {
					return {
						artifact: await arg.artifact(),
						path: [(await arg.path()).toString()],
					};
				} else if (typeof arg === "object") {
					let object: MutationMap<Apply> = {};
					if ("artifact" in arg) {
						object.artifact = arg.artifact;
					}
					if ("path" in arg) {
						object.path = await mutation({ kind: "set", value: [arg.path] });
					}
					return object;
				} else {
					return unreachable();
				}
			},
		);

		// Create the target.
		let path = relpath(path_);
		let target;
		if (artifact !== undefined && !path.isEmpty()) {
			target = await template(artifact, "/", path.toString());
		} else if (artifact !== undefined) {
			target = await template(artifact);
		} else if (!path.isEmpty()) {
			target = await template(path.toString());
		} else {
			throw new Error("Invalid symlink.");
		}

		return new Symlink({ object: { target } });
	}

	static is(value: unknown): value is Symlink {
		return value instanceof Symlink;
	}

	static expect(value: unknown): Symlink {
		assert_(Symlink.is(value));
		return value;
	}

	static assert(value: unknown): asserts value is Symlink {
		assert_(Symlink.is(value));
	}

	async id(): Promise<Symlink.Id> {
		await this.store();
		return this.#state.id!;
	}

	async object(): Promise<Symlink.Object_> {
		await this.load();
		return this.#state.object!;
	}

	async load() {
		if (this.#state.object === undefined) {
			let object = await syscall.load(this.#state.id!);
			assert_(object.kind === "symlink");
			this.#state.object = object.value;
		}
	}

	async store() {
		if (this.#state.id === undefined) {
			this.#state.id = await syscall.store({
				kind: "symlink",
				value: this.#state.object!,
			});
		}
	}

	async target(): Promise<Template> {
		return (await this.object()).target;
	}

	async artifact(): Promise<Artifact | undefined> {
		let target = await this.target();
		let firstComponent = target.components.at(0);
		if (Artifact.is(firstComponent)) {
			return firstComponent;
		} else {
			return undefined;
		}
	}

	async path(): Promise<Relpath> {
		let target = await this.target();
		let [firstComponent, secondComponent] = target.components;
		if (typeof firstComponent === "string" && secondComponent === undefined) {
			return relpath(firstComponent);
		} else if (Artifact.is(firstComponent) && secondComponent === undefined) {
			return relpath();
		} else if (
			Artifact.is(firstComponent) &&
			typeof secondComponent === "string"
		) {
			return relpath(secondComponent.slice(1));
		} else {
			throw new Error("Invalid template.");
		}
	}

	async resolve(
		from?: Unresolved<Symlink.Arg>,
	): Promise<Directory | File | undefined> {
		from = from ? await symlink(from) : undefined;
		let fromArtifact = await from?.artifact();
		if (Symlink.is(fromArtifact)) {
			fromArtifact = await fromArtifact.resolve();
		}
		let fromPath = from?.path();
		let artifact = await this.artifact();
		if (Symlink.is(artifact)) {
			artifact = await artifact.resolve();
		}
		let path = await this.path();
		if (artifact !== undefined && path.isEmpty()) {
			return artifact;
		} else if (artifact === undefined && !path.isEmpty()) {
			if (!Directory.is(fromArtifact)) {
				throw new Error("Expected a directory.");
			}
			return await fromArtifact.tryGet(
				(await (fromPath ?? relpath()))
					.parent()
					.join(path)
					.toSubpath()
					.toString(),
			);
		} else if (artifact !== undefined && !path.isEmpty()) {
			if (!Directory.is(artifact)) {
				throw new Error("Expected a directory.");
			}
			return await artifact.tryGet(path.toSubpath().toString());
		} else {
			throw new Error("Invalid symlink.");
		}
	}
}

export namespace Symlink {
	export type Arg =
		| undefined
		| string
		| Artifact
		| Template
		| Symlink
		| ArgObject
		| Array<Arg>;

	export type ArgObject = {
		artifact?: Artifact;
		path?: string;
	};

	export type Id = string;

	export type Object_ = { target: Template };

	export type State = Object_.State<Symlink.Id, Symlink.Object_>;
}
