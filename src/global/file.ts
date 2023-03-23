import { ArtifactHash, getArtifact, getArtifactHash } from "./artifact";
import { Blob, BlobHash, addBlob, getBlob } from "./blob";
import { MaybePromise } from "./resolve";
import { assert } from "./util";

export type FileLike = Uint8Array | string | File;

export let isFileLike = (fileLike: unknown): fileLike is FileLike => {
	return (
		fileLike instanceof Uint8Array ||
		typeof fileLike === "string" ||
		fileLike instanceof File
	);
};

type FileOptions = {
	executable?: boolean;
};

export let file = async (
	fileLike: MaybePromise<FileLike>,
	options?: FileOptions,
): Promise<File> => {
	fileLike = await fileLike;
	let blobHash;
	let executable;
	if (fileLike instanceof Uint8Array) {
		blobHash = await addBlob(fileLike);
	} else if (typeof fileLike === "string") {
		blobHash = await addBlob(syscall("encode_utf8", fileLike));
	} else {
		blobHash = fileLike.blobHash();
		executable = fileLike.executable();
	}
	executable = options?.executable ?? executable;
	return new File(blobHash, { executable });
};

export let isFile = (value: unknown): value is File => {
	return value instanceof File;
};

export class File {
	#blobHash: BlobHash;
	#executable: boolean;

	constructor(blobHash: BlobHash, options?: FileOptions) {
		this.#blobHash = blobHash;
		this.#executable = options?.executable ?? false;
	}

	public static async fromHash(hash: ArtifactHash): Promise<File> {
		let artifact = await getArtifact(hash);
		assert(isFile(artifact));
		return artifact;
	}

	async serialize(): Promise<syscall.File> {
		let blobHash = this.#blobHash;
		let executable = this.#executable;
		return {
			blobHash,
			executable,
		};
	}

	static async deserialize(file: syscall.File): Promise<File> {
		let blobHash = file.blobHash;
		let executable = file.executable;
		return new File(blobHash, { executable });
	}

	hash(): Promise<ArtifactHash> {
		return getArtifactHash(this);
	}

	blobHash(): BlobHash {
		return this.#blobHash;
	}

	executable(): boolean {
		return this.#executable;
	}

	async blob(): Promise<Blob> {
		return await getBlob(this.#blobHash);
	}

	async text(): Promise<string> {
		let bytes = await this.blob();
		return syscall("decode_utf8", bytes);
	}
}
