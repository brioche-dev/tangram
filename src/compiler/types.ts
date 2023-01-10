export type Diagnostic = {
	location: Location | null;
	severity: Severity;
	message: string;
};

export type Position = {
	line: number;
	character: number;
};

export type Severity = "error" | "warning" | "information" | "hint";

export type Location = {
	url: string;
	range: Range;
};

export type Range = {
	start: Position;
	end: Position;
};

export type CompletionEntry = {
	name: string;
};

export type TranspileOutput = {
	transpiled: string;
	sourceMap: string;
};
