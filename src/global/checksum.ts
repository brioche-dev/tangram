import * as syscall from "./syscall.ts";

export let checksum = (
	algorithm: Checksum.Algorithm,
	bytes: Uint8Array,
): Checksum => {
	return Checksum.new(algorithm, bytes);
};

export type Checksum = string;

export declare namespace Checksum {
	let new_: (algorithm: Checksum.Algorithm, bytes: Uint8Array) => Checksum;
	export { new_ as new };
}

export namespace Checksum {
	export type Algorithm = "blake3" | "sha256" | "sha512";

	export let new_ = (
		algorithm: Checksum.Algorithm,
		bytes: Uint8Array,
	): Checksum => {
		return syscall.checksum(algorithm, bytes);
	};
	Checksum.new = new_;
}
