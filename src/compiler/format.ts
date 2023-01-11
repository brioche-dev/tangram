import * as prettier from "prettier";

export type FormatRequest = {
	text: string;
};

export type FormatResponse = {
	text: string;
};

let prettierOptions: prettier.Options = {
	useTabs: true,
	trailingComma: "all",
};

export let format = (request: FormatRequest): FormatResponse => {
	let text = prettier.format(request.text, prettierOptions);
	return { text };
};
