export type MapLike<K extends string, V> = Record<K, V> | Map<K, V>;

export let map = <K extends string, V>(value: MapLike<K, V>): Map<K, V> => {
	if (value instanceof Map) {
		return value;
	} else {
		return new Map(Object.entries(value) as Array<[K, V]>);
	}
};
