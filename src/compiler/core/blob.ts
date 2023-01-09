import { decodeUtf8, encodeUtf8 } from "./transcode.ts";
import { MaybePromise } from "./util.ts";

export type BlobLike = string | Uint8Array | Blob;

export let isBlobLike = (blob: unknown): blob is BlobLike => {
	return (
		typeof blob === "string" ||
		blob instanceof Uint8Array ||
		blob instanceof Blob
	);
};

export let blob = async (blob: MaybePromise<BlobLike>): Promise<Blob> => {
	blob = await blob;
	if (typeof blob === "string") {
		return new Blob(encodeUtf8(blob));
	} else if (blob instanceof Uint8Array) {
		return new Blob(blob);
	} else {
		return blob;
	}
};

export class BlobHash {
	#string: string;

	constructor(string: string) {
		this.#string = string;
	}

	toString(): string {
		return this.#string;
	}
}

export class Blob {
	bytes: Uint8Array;

	constructor(bytes: Uint8Array) {
		this.bytes = bytes;
	}

	toString(): string {
		return decodeUtf8(this.bytes);
	}
}

export let addBlob = async (blob: Blob): Promise<BlobHash> => {
	return new BlobHash(await syscall("add_blob", blob.bytes));
};

export let getBlob = async (hash: BlobHash): Promise<Blob> => {
	return new Blob(await syscall("get_blob", hash.toString()));
};
