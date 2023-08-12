import { Artifact } from "./artifact.ts";
import { assert as assert_, unreachable } from "./assert.ts";
import { Block } from "./block.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Id } from "./id.ts";
import { Relpath, Subpath, relpath } from "./path.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { Template, t } from "./template.ts";
import { MaybeNestedArray, flatten } from "./util.ts";

export let symlink = async (
	...args: Array<Unresolved<Symlink.Arg>>
): Promise<Symlink> => {
	return await Symlink.new(...args);
};

type ConstructorArg = {
	block: Block;
	target: Template;
};

export class Symlink {
	#block: Block;
	#target: Template;

	constructor(arg: ConstructorArg) {
		this.#block = arg.block;
		this.#target = arg.target;
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
					} else if (Relpath.is(arg)) {
						return { path: relpath(arg) };
					} else if (Subpath.is(arg)) {
						return { path: arg.toRelpath() };
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
							artifact: arg.artifact(),
							path: arg.path(),
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
			target = await t`${artifact}/${path}`;
		} else if (artifact !== undefined) {
			target = await t`${artifact}`;
		} else if (!path.isEmpty()) {
			target = await t`${path}`;
		} else {
			throw new Error("Invalid symlink.");
		}

		return await syscall.symlink.new({ target });
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

	id(): Id {
		return this.block().id();
	}

	block(): Block {
		return this.#block;
	}

	target(): Template {
		return this.#target;
	}

	artifact(): Artifact | undefined {
		let firstComponent = this.#target.components().at(0);
		if (Artifact.is(firstComponent)) {
			return firstComponent;
		} else {
			return undefined;
		}
	}

	path(): Relpath {
		let [firstComponent, secondComponent] = this.#target.components();
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
		let fromArtifact = from?.artifact();
		if (fromArtifact instanceof Symlink) {
			fromArtifact = await fromArtifact.resolve();
		}
		let fromPath = from?.path();
		let artifact = this.artifact();
		if (artifact instanceof Symlink) {
			artifact = await artifact.resolve();
		}
		let path = this.path();
		if (artifact !== undefined && path.isEmpty()) {
			return artifact;
		} else if (artifact === undefined && !path.isEmpty()) {
			if (!(fromArtifact instanceof Directory)) {
				throw new Error("Expected a directory.");
			}
			return await fromArtifact.tryGet(
				(fromPath ?? relpath()).parent().join(path).toSubpath(),
			);
		} else if (artifact !== undefined && !path.isEmpty()) {
			if (!(artifact instanceof Directory)) {
				throw new Error("Expected a directory.");
			}
			return await artifact.tryGet(path.toSubpath());
		} else {
			throw new Error("Invalid symlink.");
		}
	}
}

export namespace Symlink {
	export type Arg =
		| string
		| Relpath
		| Subpath
		| Artifact
		| Template
		| Symlink
		| Array<Arg>
		| ArgObject;

	export type ArgObject = {
		artifact?: Artifact;
		path?: string | Subpath;
	};
}
