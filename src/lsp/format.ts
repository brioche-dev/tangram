import * as eslint from "./eslint.ts";
import * as syscall from "./syscall.ts";
import * as eslint_ from "eslint";
import prettierTypescriptPlugin from "prettier/parser-typescript";
import * as prettier from "prettier/standalone";

export type Request = {
	text: string;
};

export type Response = {
	text: string;
};

// Create the Prettier options.
let prettierOptions = {
	useTabs: true,
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
		text = syscall.resolve(prettier.format(text, prettierOptions));
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
