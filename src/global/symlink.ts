import { Artifact } from "./artifact.ts";
import { assert as assert_ } from "./assert.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Relpath, Subpath, subpath } from "./path.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { Template, t } from "./template.ts";

export let symlink = async (arg: Unresolved<Symlink.Arg>): Promise<Symlink> => {
	return await Symlink.new(arg);
};

type ConstructorArg = {
	hash: Artifact.Hash;
	target: Template;
};

export class Symlink {
	#hash: Artifact.Hash;
	#target: Template;

	static async new(arg: Unresolved<Symlink.Arg>): Promise<Symlink> {
		// Resolve the arg.
		let resolvedArg = await resolve(arg);

		// Get the artifact and path.
		let artifact: Artifact | undefined;
		let path_: string | undefined;
		if (typeof resolvedArg === "string") {
			path_ = resolvedArg;
		} else if (Relpath.is(resolvedArg) || Subpath.is(resolvedArg)) {
			path_ = resolvedArg.toString();
		} else if (Artifact.is(resolvedArg)) {
			artifact = resolvedArg;
		} else if (resolvedArg instanceof Template) {
			assert_(resolvedArg.components().length <= 2);
			let [firstComponent, secondComponent] = resolvedArg.components();
			if (typeof firstComponent === "string" && secondComponent === undefined) {
				path_ = firstComponent;
			} else if (Artifact.is(firstComponent) && secondComponent === undefined) {
				artifact = firstComponent;
			} else if (
				Artifact.is(firstComponent) &&
				typeof secondComponent === "string"
			) {
				artifact = firstComponent;
				assert_(secondComponent.startsWith("/"));
				path_ = secondComponent.slice(1);
			} else {
				throw new Error("Invalid template.");
			}
		} else if (resolvedArg instanceof Symlink) {
			return resolvedArg;
		} else if (typeof resolvedArg === "object") {
			artifact = resolvedArg.artifact;
			let resolvedArgPath = resolvedArg.path;
			if (typeof resolvedArgPath === "string") {
				path_ = resolvedArgPath;
			} else if (Subpath.is(resolvedArgPath)) {
				path_ = resolvedArgPath.toString();
			}
		}

		// Create the target.
		let target;
		if (artifact !== undefined && path_ !== undefined) {
			target = await t`${artifact}/${path_}`;
		} else if (artifact !== undefined && path_ === undefined) {
			target = await t`${artifact}`;
		} else if (artifact === undefined && path_ !== undefined) {
			target = await t`${path_}`;
		} else {
			target = await t``;
		}

		return Symlink.fromSyscall(
			syscall.symlink.new({ target: target.toSyscall() }),
		);
	}

	constructor(arg: ConstructorArg) {
		this.#hash = arg.hash;
		this.#target = arg.target;
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

	toSyscall(): syscall.Symlink {
		let hash = this.#hash;
		let target = this.#target.toSyscall();
		return {
			hash,
			target,
		};
	}

	static fromSyscall(symlink: syscall.Symlink): Symlink {
		let hash = symlink.hash;
		let target = Template.fromSyscall(symlink.target);
		return new Symlink({
			hash,
			target,
		});
	}

	hash(): Artifact.Hash {
		return this.#hash;
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

	path(): Subpath {
		let [firstComponent, secondComponent] = this.#target.components();
		if (typeof firstComponent === "string" && secondComponent === undefined) {
			return subpath(firstComponent);
		} else if (Artifact.is(firstComponent) && secondComponent === undefined) {
			return subpath();
		} else if (
			Artifact.is(firstComponent) &&
			typeof secondComponent === "string"
		) {
			return subpath(secondComponent);
		} else {
			throw new Error("Invalid template.");
		}
	}

	async resolve(): Promise<Directory | File | undefined> {
		let result: Artifact = this;
		while (Symlink.is(result)) {
			let artifact = result.artifact();
			let path = result.path();
			if (Directory.is(artifact)) {
				result = await artifact.get(path);
			} else if (File.is(artifact)) {
				assert_(path.components().length === 0);
				result = artifact;
			} else if (Symlink.is(artifact)) {
				assert_(path.components().length === 0);
				result = artifact;
			} else {
				throw new Error(
					"Cannot resolve a symlink without an artifact in its target.",
				);
			}
		}
		return result;
	}
}

export namespace Symlink {
	export type Arg =
		| string
		| Relpath
		| Subpath
		| Artifact
		| Template
		| ArgObject;

	export type ArgObject = {
		artifact?: Artifact;
		path?: string | Subpath;
	};
}
