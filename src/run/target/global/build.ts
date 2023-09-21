import { assert as assert_ } from "./assert.ts";
import { Resource } from "./resource.ts";
import { Target } from "./target.ts";
import { Task } from "./task.ts";

export type Build = Resource | Target | Task;

export namespace Build {
	export let is = (value: unknown): value is Build => {
		return (
			value instanceof Resource ||
			value instanceof Target ||
			value instanceof Task
		);
	};

	export let expect = (value: unknown): Build => {
		assert_(is(value));
		return value;
	};

	export let assert = (value: unknown): asserts value is Build => {
		assert_(is(value));
	};
}
