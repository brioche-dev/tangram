import { Artifact } from "./artifact.ts";
import { assert as assert_, unreachable } from "./assert.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Object_ } from "./object.ts";
import { Relpath, relpath } from "./path.ts";
import { Unresolved, resolve } from "./resolve.ts";
import { Template, t } from "./template.ts";
import { MaybeNestedArray, flatten } from "./util.ts";

export let symlink = async (
	...args: Array<Unresolved<Symlink.Arg>>
): Promise<Symlink> => {
	return await Symlink.new(...args);
};

export class Symlink {
	#handle: Object_.Handle;

	constructor(handle: Object_.Handle) {
		this.#handle = handle;
	}

	static async new(...args: Array<Unresolved<Symlink.Arg>>): Promise<Symlink> {
		// Get the artifact and path.
		let { artifact, path } = flatten(
			await Promise.all(
				args.map(async function map(
					unresolvedArg,
				): Promise<MaybeNestedArray<{ artifact?: Artifact; path?: Relpath }>> {
					let arg = await resolve(unresolvedArg);
					if (typeof arg === "string") {
						return { path: relpath(arg) };
					} else if (Artifact.is(arg)) {
						return { artifact: arg };
					} else if (arg instanceof Template) {
						assert_(arg.components().length <= 2);
						let [firstComponent, secondComponent] = arg.components();
						if (
							typeof firstComponent === "string" &&
							secondComponent === undefined
						) {
							return { path: relpath(firstComponent) };
						} else if (
							Artifact.is(firstComponent) &&
							secondComponent === undefined
						) {
							return { artifact: firstComponent };
						} else if (
							Artifact.is(firstComponent) &&
							typeof secondComponent === "string"
						) {
							assert_(secondComponent.startsWith("/"));
							return {
								artifact: firstComponent,
								path: relpath(secondComponent.slice(1)),
							};
						} else {
							throw new Error("Invalid template.");
						}
					} else if (arg instanceof Symlink) {
						return {
							artifact: await arg.artifact(),
							path: await arg.path(),
						};
					} else if (arg instanceof Array) {
						return await Promise.all(arg.map(map));
					} else if (typeof arg === "object") {
						return {
							artifact: arg.artifact,
							path: relpath(arg.path),
						};
					} else {
						return unreachable();
					}
				}),
			),
		).reduce<{ artifact: Artifact | undefined; path: Relpath }>(
			(value, { artifact, path }) => {
				if (artifact !== undefined) {
					value.artifact = artifact;
					value.path = path ?? relpath();
				} else {
					value.path = value.path.join(path);
				}
				return value;
			},
			{ artifact: undefined, path: relpath() },
		);

		// Create the target.
		let target;
		if (artifact !== undefined && !path.isEmpty()) {
			target = await t`${artifact}/${path.toString()}`;
		} else if (artifact !== undefined) {
			target = await t`${artifact}`;
		} else if (!path.isEmpty()) {
			target = await t`${path.toString()}`;
		} else {
			throw new Error("Invalid symlink.");
		}

		return new Symlink(
			Object_.Handle.withObject({ kind: "symlink", value: { target } }),
		);
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
		return (await this.#handle.id()) as Symlink.Id;
	}

	async object(): Promise<Symlink.Object_> {
		let object = await this.#handle.object();
		assert_(object.kind === "symlink");
		return object.value;
	}

	handle(): Object_.Handle {
		return this.#handle;
	}

	async target(): Promise<Template> {
		return (await this.object()).target;
	}

	async artifact(): Promise<Artifact | undefined> {
		let target = await this.target();
		let firstComponent = target.components().at(0);
		if (Artifact.is(firstComponent)) {
			return firstComponent;
		} else {
			return undefined;
		}
	}

	async path(): Promise<Relpath> {
		let target = await this.target();
		let [firstComponent, secondComponent] = target.components();
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
		if (fromArtifact instanceof Symlink) {
			fromArtifact = await fromArtifact.resolve();
		}
		let fromPath = from?.path();
		let artifact = await this.artifact();
		if (artifact instanceof Symlink) {
			artifact = await artifact.resolve();
		}
		let path = await this.path();
		if (artifact !== undefined && path.isEmpty()) {
			return artifact;
		} else if (artifact === undefined && !path.isEmpty()) {
			if (!(fromArtifact instanceof Directory)) {
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
			if (!(artifact instanceof Directory)) {
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
		| string
		| Artifact
		| Template
		| Symlink
		| Array<Arg>
		| ArgObject;

	export type ArgObject = {
		artifact?: Artifact;
		path?: string;
	};

	export type Id = string;

	export type Object_ = { target: Template };
}
