export let inspect = {
	custom: Symbol("util.inspect.custom"),
};

export let promisify = (f) => {
	return () => {
		throw new Error("unimplemented");
	};
};
