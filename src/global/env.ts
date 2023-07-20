import { assert } from "./assert";
import { Value } from "./value";

export namespace env {
	export let value: Record<string, Value> | undefined;

	export let get = (): Record<string, Value> => {
		assert(value);
		return value;
	};
}
