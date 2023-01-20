import "./syscall";
import { Artifact } from "./artifact";
import { Download } from "./download";
import { Process } from "./process";
import { Target } from "./target";
import { deserializeValue } from "./value";

export type OperationType = "download" | "process" | "target";

export type Operation = Download | Process | Target;

export type Output<T extends Operation> = T extends Download
	? Artifact
	: T extends Process
	? Artifact
	: T extends Target<infer U>
	? U
	: never;

export let run = async <T extends Operation>(
	operation: T,
): Promise<Output<T>> => {
	let operationSerialized = await serializeOperation(operation);
	let outputSerialized = await syscall("run", operationSerialized);
	let output = await deserializeValue(outputSerialized);
	return output as Output<T>;
};

export let serializeOperation = async (
	operation: Operation,
): Promise<syscall.Operation> => {
	if (operation instanceof Download) {
		return {
			type: "download",
			value: await operation.serialize(),
		};
	} else if (operation instanceof Process) {
		return {
			type: "process",
			value: await operation.serialize(),
		};
	} else if (operation instanceof Target) {
		return {
			type: "target",
			value: await operation.serialize(),
		};
	} else {
		throw new Error("Cannot serialize operation.");
	}
};

export let deserializeOperation = async (
	operation: syscall.Operation,
): Promise<Operation> => {
	switch (operation.type) {
		case "download": {
			return await Download.deserialize(operation.value);
		}
		case "process": {
			return await Process.deserialize(operation.value);
		}
		case "target": {
			return await Target.deserialize(operation.value);
		}
	}
};
