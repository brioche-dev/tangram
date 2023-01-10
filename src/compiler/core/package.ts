import { Artifact, ArtifactHash, addArtifact } from "./artifact.ts";

export let currentPackage = async (): Promise<Package> => {
	return await getPackage(new PackageHash(syscall("get_current_package_hash")));
};

export type PackageArgs = {
	source: Artifact;
	dependencies: { [name: string]: Package };
};

export let package_ = async (args: PackageArgs): Promise<Package> => {
	let source = await addArtifact(await args.source);
	let dependencies = Object.fromEntries(
		await Promise.all(
			Object.entries(args.dependencies).map(async ([key, value]) => [
				key,
				await addPackage(value),
			]),
		),
	);
	return new Package({
		source,
		dependencies,
	});
};

export class PackageHash {
	#string: string;

	constructor(string: string) {
		this.#string = string;
	}

	toString(): string {
		return this.#string;
	}
}

type PackageConstructorArgs = {
	source: ArtifactHash;
	dependencies: { [key: string]: PackageHash };
};

export class Package {
	source: ArtifactHash;
	dependencies: { [key: string]: PackageHash };

	constructor({ source, dependencies }: PackageConstructorArgs) {
		this.source = source;
		this.dependencies = dependencies;
	}

	async serialize(): Promise<syscall.Package> {
		let source = this.source.toString();
		let dependencies = Object.fromEntries(
			await Promise.all(
				Object.entries(this.dependencies).map(async ([key, value]) => [
					key,
					value.toString(),
				]),
			),
		);
		return {
			source,
			dependencies,
		};
	}

	static async deserialize(package_: syscall.Package): Promise<Package> {
		let source = new ArtifactHash(package_.source);
		let dependencies = Object.fromEntries(
			Object.entries(package_.dependencies).map(([key, value]) => [
				key,
				new PackageHash(value),
			]),
		);
		return new Package({
			source,
			dependencies,
		});
	}
}

export let addPackage = async (package_: Package): Promise<PackageHash> => {
	return new PackageHash(
		await syscall("add_package", await package_.serialize()),
	);
};

export let getPackage = async (hash: PackageHash): Promise<Package> => {
	return await Package.deserialize(
		await syscall("get_package", hash.toString()),
	);
};
