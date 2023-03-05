export type Checksum = `${ChecksumAlgorithm}${":" | "-"}${string}`;

export type ChecksumAlgorithm = "blake3" | "sha256";

export let checksum = (
	algorithm: ChecksumAlgorithm,
	bytes: Uint8Array | string,
): Checksum => {
	return syscall("checksum", algorithm, bytes);
};
