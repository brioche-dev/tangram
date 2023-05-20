import { unreachable } from "./assert.ts";
import { Command } from "./command.ts";
import { Function } from "./function.ts";
import { Resource } from "./resource.ts";
import * as syscall from "./syscall.ts";

export type Operation = Command | Function | Resource;

export namespace Operation {
	export type Hash = string;

	export let is = (value: unknown): value is Operation => {
		return (
			value instanceof Command ||
			value instanceof Function ||
			value instanceof Resource
		);
	};

	export let toSyscall = (operation: Operation): syscall.Operation => {
		if (operation instanceof Command) {
			return {
				kind: "command",
				value: operation.toSyscall(),
			};
		} else if (operation instanceof Function) {
			return {
				kind: "function",
				value: operation.toSyscall(),
			};
		} else if (operation instanceof Resource) {
			return {
				kind: "resource",
				value: operation.toSyscall(),
			};
		} else {
			return unreachable();
		}
	};

	export let fromSyscall = (operation: syscall.Operation): Operation => {
		switch (operation.kind) {
			case "command": {
				return Command.fromSyscall(operation.value);
			}
			case "function": {
				return Function.fromSyscall(operation.value);
			}
			case "resource": {
				return Resource.fromSyscall(operation.value);
			}
			default: {
				return unreachable();
			}
		}
	};
}
