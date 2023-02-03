export class URL {
	constructor(url, base) {}
}

export let pathToFileURL = (path) => {
	return new URL(`file://${path}`);
};
