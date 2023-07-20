import { unreachable } from "./assert.ts";
import { Resource } from "./resource.ts";
import * as syscall from "./syscall.ts";
import { Target } from "./target.ts";
import { Task } from "./task.ts";

export type Operation = Resource | Target | Task;

export namespace Operation {
	export let is = (value: unknown): value is Operation => {
		return (
			value instanceof Resource ||
			value instanceof Target ||
			value instanceof Task
		);
	};

	export let toSyscall = (operation: Operation): syscall.Operation => {
		if (operation instanceof Resource) {
			return {
				kind: "resource",
				value: operation.toSyscall(),
			};
		} else if (operation instanceof Target) {
			return {
				kind: "target",
				value: operation.toSyscall(),
			};
		} else if (operation instanceof Task) {
			return {
				kind: "task",
				value: operation.toSyscall(),
			};
		} else {
			return unreachable();
		}
	};

	export let fromSyscall = (operation: syscall.Operation): Operation => {
		switch (operation.kind) {
			case "resource": {
				return Resource.fromSyscall(operation.value);
			}
			case "target": {
				return Target.fromSyscall(operation.value);
			}
			case "task": {
				return Task.fromSyscall(operation.value);
			}
			default: {
				return unreachable();
			}
		}
	};
}
