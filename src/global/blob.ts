export type BlobHash = string;

export type Blob = Uint8Array;

export let addBlob = async (blob: Blob): Promise<BlobHash> => {
	return await syscall("add_blob", blob);
};

export let getBlob = async (hash: BlobHash): Promise<Blob> => {
	return await syscall("get_blob", hash);
};
