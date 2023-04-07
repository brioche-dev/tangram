export type Checksum = `${Checksum.Algorithm}${":" | "-"}${string}`;

export namespace Checksum {
	export type Algorithm = "blake3" | "sha256";
}
