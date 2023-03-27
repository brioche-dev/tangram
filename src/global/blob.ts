import { MaybePromise } from "./resolve";
import * as syscall from "./syscall";

export type BlobHash = string;

export type BlobLike = Uint8Array | string | Blob;

export let isBlobLike = (value: unknown): value is BlobLike => {
	return (
		value instanceof Uint8Array ||
		typeof value === "string" ||
		value instanceof Blob
	);
};

export let blob = async (blobLike: MaybePromise<BlobLike>): Promise<Blob> => {
	blobLike = await blobLike;
	if (blobLike instanceof Uint8Array) {
		return new Blob(await addBlob(blobLike));
	} else if (typeof blobLike === "string") {
		let bytes = syscall.encodeUtf8(blobLike);
		return new Blob(await addBlob(bytes));
	} else {
		return blobLike;
	}
};

export class Blob {
	#hash: BlobHash;

	constructor(hash: BlobHash) {
		this.#hash = hash;
	}

	hash(): BlobHash {
		return this.#hash;
	}

	async bytes(): Promise<Uint8Array> {
		return await getBlob(this.#hash);
	}

	async text(): Promise<string> {
		let bytes = await this.bytes();
		return syscall.decodeUtf8(bytes);
	}
}

export let addBlob = async (bytes: Uint8Array): Promise<BlobHash> => {
	return await syscall.addBlob(bytes);
};

export let getBlob = async (hash: BlobHash): Promise<Uint8Array> => {
	return await syscall.getBlob(hash);
};
