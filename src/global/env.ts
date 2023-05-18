import { assert } from "./assert";
import { Value } from "./value";

type Env = {
	value?: Record<string, Value>;
	get(): Record<string, Value>;
};

export let env: Env = {
	get() {
		assert(this.value);
		return this.value;
	},
};
