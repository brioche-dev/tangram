import {
	Artifact,
	ArtifactHash,
	getArtifact,
	serializeArtifact,
} from "./artifact";
import { Blob, BlobHash, BlobLike, blob, getBlob, isBlobLike } from "./blob";
import { MaybePromise } from "./resolve";
import * as syscall from "./syscall";

export type FileLike = BlobLike | File;

export let isFileLike = (value: unknown): value is FileLike => {
	return isBlobLike(value) || value instanceof File;
};

export type FileArg = MaybePromise<BlobLike | File | FileObject>;

export type FileObject = {
	blob: MaybePromise<BlobLike>;
	executable?: boolean;
	references?: Array<MaybePromise<Artifact>>;
};

export let file = async (arg: FileArg): Promise<File> => {
	arg = await arg;
	if (isBlobLike(arg)) {
		return new File({
			blobHash: (await blob(arg)).hash(),
			executable: false,
			references: [],
		});
	} else if (isFile(arg)) {
		return arg;
	} else {
		let blobHash = (await blob(arg.blob)).hash();
		let executable = arg.executable ?? false;
		let references = await Promise.all(
			(arg.references ?? []).map(async (reference) => {
				reference = await reference;
				return await reference.hash();
			}),
		);
		return new File({ blobHash, executable, references });
	}
};

export let isFile = (value: unknown): value is File => {
	return value instanceof File;
};

type FileConstructorArgs = {
	blobHash: BlobHash;
	executable: boolean;
	references: Array<ArtifactHash>;
};

export class File {
	#blobHash: BlobHash;
	#executable: boolean;
	#references: Array<ArtifactHash>;

	constructor(args: FileConstructorArgs) {
		this.#blobHash = args.blobHash;
		this.#executable = args.executable;
		this.#references = args.references;
	}

	async serialize(): Promise<syscall.File> {
		let blobHash = this.#blobHash;
		let executable = this.#executable;
		let references = this.#references;
		return {
			blobHash,
			executable,
			references,
		};
	}

	static async deserialize(file: syscall.File): Promise<File> {
		let blobHash = file.blobHash;
		let executable = file.executable;
		let references = file.references;
		return new File({ blobHash, executable, references });
	}

	async hash(): Promise<ArtifactHash> {
		return syscall.addArtifact(await serializeArtifact(this));
	}

	blobHash(): BlobHash {
		return this.#blobHash;
	}

	blob(): Blob {
		return new Blob(this.#blobHash);
	}

	async bytes(): Promise<Uint8Array> {
		return await this.blob().bytes();
	}

	async text(): Promise<string> {
		return await this.blob().text();
	}

	executable(): boolean {
		return this.#executable;
	}

	async references(): Promise<Array<Artifact>> {
		return await Promise.all(this.#references.map(getArtifact));
	}
}
