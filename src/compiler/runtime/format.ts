import * as prettier from "prettier";
import { FormatRequest, FormatResponse } from "./types";

let prettierOptions: prettier.Options = {
	useTabs: true,
	trailingComma: "all",
};

export let format = (request: FormatRequest): FormatResponse => {
	let text = prettier.format(request.text, prettierOptions);
	return { text };
};
