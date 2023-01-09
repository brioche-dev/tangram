import { Artifact } from "./artifact.ts";
import { Blob, BlobHash, BlobLike, addBlob, blob, getBlob } from "./blob.ts";

export type FileOptions = {
	executable?: boolean;
};

export let file = async (
	blobLike: BlobLike,
	options?: FileOptions,
): Promise<File> => {
	let blobHash = await addBlob(await blob(blobLike));
	return new File(blobHash, options);
};

export class File {
	blob: BlobHash;
	executable: boolean;

	constructor(blob: BlobHash, options?: FileOptions) {
		this.blob = blob;
		this.executable = options?.executable ?? false;
	}

	async serialize(): Promise<syscall.File> {
		let blob = this.blob.toString();
		let executable = this.executable;
		return {
			blob,
			executable,
		};
	}

	static async deserialize(file: syscall.File): Promise<File> {
		let blob = new BlobHash(file.blob);
		let executable = file.executable;
		return new File(blob, { executable });
	}

	async getBlob(): Promise<Blob> {
		return await getBlob(this.blob);
	}
}
