import { assert } from "./util";
import { Value, deserializeValue, isNullish, serializeValue } from "./value";

class Context {
	async get(key: string): Promise<Value | undefined> {
		let serializedValue = syscall("get_context_value", key);
		if (isNullish(serializedValue)) {
			return undefined;
		}
		let value = await deserializeValue(serializedValue);
		return value;
	}

	async set(key: string, value: Value): Promise<void> {
		let serializedValue = await serializeValue(value);
		syscall("set_context_value", key, serializedValue);
	}

	async entries(): Promise<Array<[string, Value]>> {
		let keys = syscall("get_context_keys");
		let entries = await Promise.all(
			keys.map(async (key): Promise<[string, Value]> => {
				let value = await this.get(key);
				assert(value);
				return [key, value];
			}),
		);
		return entries;
	}
}

export let context = new Context();
