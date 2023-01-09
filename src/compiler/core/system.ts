import { unreachable } from "./util.ts";

export type Arch = "amd64" | "arm64";

export type Os = "linux" | "macos";

export type System =
	| "amd64_linux"
	| "arm64_linux"
	| "amd64_macos"
	| "arm64_macos";

export let archForSystem = (system: System): Arch => {
	switch (system) {
		case "amd64_linux":
		case "amd64_macos": {
			return "amd64";
		}
		case "arm64_linux":
		case "arm64_macos": {
			return "arm64";
		}
		default: {
			throw new Error("Invalid system.");
		}
	}
};

export let osForSystem = (system: System): Os => {
	switch (system) {
		case "amd64_linux":
		case "arm64_linux": {
			return "linux";
		}
		case "amd64_macos":
		case "arm64_macos": {
			return "macos";
		}
		default: {
			throw new Error("Invalid system.");
		}
	}
};

export type SystemFromPartsArgs = {
	arch: Arch;
	os: Os;
};

export let systemFromParts = (args: SystemFromPartsArgs): System => {
	let { arch, os } = args;
	if (arch === "amd64" && os === "linux") {
		return "amd64_linux";
	}
	if (arch === "arm64" && os === "linux") {
		return "arm64_linux";
	}
	if (arch === "amd64" && os === "macos") {
		return "amd64_macos";
	}
	if (arch === "arm64" && os === "macos") {
		return "arm64_macos";
	}
	return unreachable(args);
};
