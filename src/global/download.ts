import { Artifact } from "./artifact";
import { Checksum } from "./checksum";
import { run } from "./operation";
import * as syscall from "./syscall";
import { nullish } from "./value";

export type DownloadArgs = {
	url: string;
	unpack?: boolean | nullish;
	checksum?: Checksum | nullish;
	unsafe?: boolean | nullish;
};

export let download = async (args: DownloadArgs): Promise<Artifact> => {
	return await new Download(args).run();
};

export let isDownload = (value: unknown): value is Download => {
	return value instanceof Download;
};

export class Download {
	#url: string;
	#unpack: boolean;
	#checksum: Checksum | nullish;
	#unsafe: boolean;

	constructor(args: DownloadArgs) {
		this.#url = args.url;
		this.#unpack = args.unpack ?? false;
		this.#checksum = args.checksum ?? null;
		this.#unsafe = args.unsafe ?? false;
	}

	async serialize(): Promise<syscall.Download> {
		return {
			url: this.#url,
			unpack: this.#unpack,
			checksum: this.#checksum,
			unsafe: this.#unsafe,
		};
	}

	static async deserialize(download: syscall.Download): Promise<Download> {
		return new Download({
			url: download.url,
			unpack: download.unpack,
			checksum: download.checksum,
			unsafe: download.unsafe,
		});
	}

	async run(): Promise<Artifact> {
		return await run(this);
	}
}
