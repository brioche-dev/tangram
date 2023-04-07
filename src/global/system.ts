export namespace System {
	export type Arg = System | ArgObject;

	export type ArgObject = {
		arch: Arch;
		os: Os;
	};
}

export let system = (arg: System.Arg): System => {
	if (typeof arg === "string") {
		return arg;
	} else {
		let { arch, os } = arg;
		return `${arch}_${os}` as System;
	}
};

export type System =
	| "amd64_linux"
	| "arm64_linux"
	| "amd64_macos"
	| "arm64_macos";

export namespace System {
	export type Arch = "amd64" | "arm64";

	export type Os = "linux" | "macos";

	export let arch = (system: System): Arch => {
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

	export let os = (system: System): Os => {
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
}
