let posix = {
	dirname: () => {
		throw new Error("Unimplemented.");
	},
};

let isAbsolute = (path) => {
	return path.startsWith("/");
};

let join = (...paths) => {
	return paths.join("/");
};

let extname = (path) => {
	let parts = path.split(".");
	return parts.at(-1) ?? "";
};

module.exports = { isAbsolute, join, posix, extname };
