import * as eslint from "./eslint.ts";
import * as eslint_ from "eslint";
import * as prettier from "prettier";
// @ts-ignore
import prettierTypescriptPlugin from "prettier/esm/parser-typescript.mjs";

export type Request = {
	text: string;
};

export type Response = {
	text: string;
};

// Create the Prettier options.
let prettierOptions: prettier.Options = {
	useTabs: true,
	trailingComma: "all",
	parser: "typescript",
	plugins: [prettierTypescriptPlugin],
};

// Create the ESLint config.
let eslintConfig: eslint_.Linter.Config = {
	rules: {
		"@tangramdotdev/sort-imports": "warn",
		"@tangramdotdev/template-indent": "warn",
		"sort-imports": ["warn", { ignoreDeclarationSort: true }],
		semi: "warn",
	},
	parser: "@typescript-eslint/parser",
};

export let handle = (request: Request): Response => {
	let text = request.text;

	// Format the text with Prettier.
	try {
		text = prettier.format(text, prettierOptions);
	} catch {
		// Ignore errors.
	}

	try {
		// Run ESLint and fix any errors.
		let eslintOutput = eslint.linter.verifyAndFix(text, eslintConfig);
		text = eslintOutput.output;
	} catch (e) {
		// Ignore errors.
	}

	return { text };
};
