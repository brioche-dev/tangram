import { Artifact } from "./artifact.ts";
import { Blob } from "./blob.ts";
import { Checksum } from "./checksum.ts";
import * as syscall from "./syscall.ts";

export let download = async (
	url: string,
	checksum: Checksum,
): Promise<Blob> => {
	return await syscall.download(url, checksum);
};

export let unpack = async (
	blob: Blob,
	format: UnpackFormat,
): Promise<Artifact> => {
	return await syscall.unpack(blob, format);
};

export type UnpackFormat =
	| ".tar"
	| ".tar.bz2"
	| ".tar.gz"
	| ".tar.lz"
	| ".tar.xz"
	| ".tar.zstd"
	| ".zip";
