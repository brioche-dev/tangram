import * as syscall from "./syscall";

export type BlobHash = string;

export type Blob = Uint8Array;

export let addBlob = async (blob: Blob): Promise<BlobHash> => {
	return await syscall.addBlob(blob);
};

export let getBlob = async (hash: BlobHash): Promise<Blob> => {
	return await syscall.getBlob(hash);
};
