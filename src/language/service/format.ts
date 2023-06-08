import templateIndent from "./template_indent.ts";
import * as tangramEslintPlugin from "@tangramdotdev/eslint-plugin";
import * as typescriptEslintParser from "@typescript-eslint/parser";
import * as eslint from "eslint";
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

// Create an ESLint linter.
let linter = new eslint.Linter();

// Use the TypeScript ESLint parser.
linter.defineParser(
	"@typescript-eslint/parser",
	typescriptEslintParser as eslint.Linter.ParserModule,
);

// Define the tangram rules.
linter.defineRules({
	"@tangramdotdev/sort-imports": tangramEslintPlugin.rules["sort-imports"]!,
	"@tangramdotdev/template-indent": templateIndent,
});

// Create the ESLint config.
let eslintConfig: eslint.Linter.Config = {
	rules: {
		"@tangramdotdev/sort-imports": "warn",
		"@tangramdotdev/template-indent": "warn",
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
		let eslintOutput = linter.verifyAndFix(text, eslintConfig);
		text = eslintOutput.output;
	} catch (e) {
		// Ignore errors.
	}

	return { text };
};
