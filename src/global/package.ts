import { Artifact, ArtifactHash, addArtifact, getArtifact } from "./artifact";

export type PackageInstanceHash = string;

export type PackageInstanceArgs = {
	package: Artifact;
	dependencies: Record<string, PackageInstance>;
};

export let packageInstance = async (
	args: PackageInstanceArgs,
): Promise<PackageInstance> => {
	let packageHash = await addArtifact(await args.package);
	let dependencies = Object.fromEntries(
		await Promise.all(
			Object.entries(args.dependencies).map(async ([key, value]) => [
				key,
				await addPackageInstance(value),
			]),
		),
	);
	return new PackageInstance({
		packageHash,
		dependencies,
	});
};

type PackageInstanceConstructorArgs = {
	packageHash: ArtifactHash;
	dependencies: Record<string, PackageInstanceHash>;
};

export class PackageInstance {
	#packageHash: ArtifactHash;
	#dependencies: Record<string, PackageInstanceHash>;

	constructor(args: PackageInstanceConstructorArgs) {
		this.#packageHash = args.packageHash;
		this.#dependencies = args.dependencies;
	}

	async serialize(): Promise<syscall.PackageInstance> {
		let packageHash = this.#packageHash;
		let dependencies = Object.fromEntries(
			Object.entries(this.#dependencies).map(([key, value]) => [key, value]),
		);
		return {
			packageHash,
			dependencies,
		};
	}

	static deserialize(
		packageInstance: syscall.PackageInstance,
	): PackageInstance {
		let packageHash = packageInstance.packageHash;
		let dependencies = Object.fromEntries(
			Object.entries(packageInstance.dependencies).map(([key, value]) => [
				key,
				value,
			]),
		);
		return new PackageInstance({
			packageHash,
			dependencies,
		});
	}

	async getPackage(): Promise<Artifact> {
		return await getArtifact(this.#packageHash);
	}

	async getDependencies(): Promise<Record<string, PackageInstance>> {
		throw new Error("Unimplemented.");
	}
}

export let addPackageInstance = async (
	packageInstance: PackageInstance,
): Promise<PackageInstanceHash> => {
	return await syscall(
		"add_package_instance",
		await packageInstance.serialize(),
	);
};

export let getPackageInstance = async (
	hash: PackageInstanceHash,
): Promise<PackageInstance> => {
	return PackageInstance.deserialize(
		await syscall("get_package_instance", hash),
	);
};
