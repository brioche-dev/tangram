import { Resource } from "./resource.ts";
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
}
