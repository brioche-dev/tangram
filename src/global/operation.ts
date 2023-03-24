import { Artifact } from "./artifact";
import { Call } from "./call";
import { Download } from "./download";
import { Process } from "./process";
import * as syscall from "./syscall";
import { deserializeValue } from "./value";

export type OperationHash = string;

export type OperationKind = "download" | "process" | "call";

export type Operation = Download | Process | Call;

export type Output<T extends Operation> = T extends Download
	? Artifact
	: T extends Process
	? Artifact
	: T extends Call<infer U>
	? U
	: never;

export let isOperation = (value: unknown): value is Operation => {
	return (
		value instanceof Download ||
		value instanceof Process ||
		value instanceof Call
	);
};

export let run = async <T extends Operation>(
	operation: T,
): Promise<Output<T>> => {
	let operationHash = await addOperation(operation);
	let outputSerialized = await syscall.runOperation(operationHash);
	let output = await deserializeValue(outputSerialized);
	return output as Output<T>;
};

export let addOperation = async (
	operation: Operation,
): Promise<OperationHash> => {
	return await syscall.addOperation(await serializeOperation(operation));
};

export let getArtifact = async (hash: OperationHash): Promise<Operation> => {
	return await deserializeOperation(await syscall.getOperation(hash));
};

export let serializeOperation = async (
	operation: Operation,
): Promise<syscall.Operation> => {
	if (operation instanceof Download) {
		return {
			kind: "download",
			value: await operation.serialize(),
		};
	} else if (operation instanceof Process) {
		return {
			kind: "process",
			value: await operation.serialize(),
		};
	} else if (operation instanceof Call) {
		return {
			kind: "call",
			value: await operation.serialize(),
		};
	} else {
		throw new Error("Cannot serialize operation.");
	}
};

export let deserializeOperation = async (
	operation: syscall.Operation,
): Promise<Operation> => {
	switch (operation.kind) {
		case "download": {
			return await Download.deserialize(operation.value);
		}
		case "process": {
			return await Process.deserialize(operation.value);
		}
		case "call": {
			return await Call.deserialize(operation.value);
		}
	}
};
