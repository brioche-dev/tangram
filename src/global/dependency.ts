import "./syscall";
import {
	Artifact,
	ArtifactHash,
	addArtifact,
	getArtifact,
	isArtifact,
} from "./artifact";
import { MaybePromise } from "./util";

export type DependencyArgs = {
	artifact: MaybePromise<Artifact>;
	path?: string | null | undefined;
};

export let dependency = async (args: DependencyArgs): Promise<Dependency> => {
	let artifact = await addArtifact(await args.artifact);
	let path = args.path;
	return new Dependency({
		artifact,
		path,
	});
};

export type DependencyConstructorArgs = {
	artifact: ArtifactHash;
	path?: string | null | undefined;
};

export class Dependency {
	artifact: ArtifactHash;
	path: string | null | undefined;

	constructor({ artifact, path }: DependencyConstructorArgs) {
		this.artifact = artifact;
		this.path = path;
	}

	static isDependency(value: unknown): value is Dependency {
		return value instanceof Dependency;
	}

	async serialize(): Promise<syscall.Dependency> {
		let artifact = this.artifact.toString();
		let path = this.path;
		return {
			artifact,
			path,
		};
	}

	static async deserialize(
		dependency: syscall.Dependency,
	): Promise<Dependency> {
		return new Dependency({
			artifact: new ArtifactHash(dependency.artifact),
			path: dependency.path,
		});
	}

	async getArtifact(): Promise<Artifact> {
		let artifact = await getArtifact(this.artifact);
		if (!isArtifact(artifact)) {
			throw new Error("The value is not an artifact.");
		}
		return artifact;
	}
}
