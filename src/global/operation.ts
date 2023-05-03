import { unreachable } from "./assert.ts";
import { Call } from "./call.ts";
import { Download } from "./download.ts";
import { Process } from "./process.ts";
import * as syscall from "./syscall.ts";

export type Operation = Call | Download | Process;

export namespace Operation {
	export type Hash = string;

	export type Kind = "download" | "process" | "call";

	export let is = (value: unknown): value is Operation => {
		return (
			value instanceof Call ||
			value instanceof Download ||
			value instanceof Process
		);
	};

	export let toSyscall = (operation: Operation): syscall.Operation => {
		if (operation instanceof Download) {
			return {
				kind: "download",
				value: operation.toSyscall(),
			};
		} else if (operation instanceof Process) {
			return {
				kind: "process",
				value: operation.toSyscall(),
			};
		} else if (operation instanceof Call) {
			return {
				kind: "call",
				value: operation.toSyscall(),
			};
		} else {
			return unreachable();
		}
	};

	export let fromSyscall = (
		hash: Operation.Hash,
		operation: syscall.Operation,
	): Operation => {
		switch (operation.kind) {
			case "download": {
				return Download.fromSyscall(operation.value);
			}
			case "process": {
				return Process.fromSyscall(operation.value);
			}
			case "call": {
				return Call.fromSyscall(operation.value);
			}
			default: {
				return unreachable();
			}
		}
	};
}
