export type MaybeArray<T> = T | Array<T>;

export type NestedArray<T> = Array<MaybeNestedArray<T>>;

export type MaybeNestedArray<T> = T | Array<MaybeNestedArray<T>>;

export let flatten = <T>(value: NestedArray<T>): Array<T> => {
	// @ts-ignore
	return value.flat(Infinity);
};
