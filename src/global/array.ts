export type MaybeArray<T> = T | Array<T>;

export type ArrayLike<T> = Iterable<T> | Array<T>;

export let array = <T>(value: ArrayLike<T>): Array<T> => {
	return Array.from(value);
};
