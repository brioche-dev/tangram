export type MaybeArray<T> = T | Array<T>;

export type MaybePromise<T> = T | PromiseLike<T>;

export let unreachable = (value: any): never => {
	throw new Error(`Reached unreachable code: "${value}".`);
};
