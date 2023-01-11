import { Location, CompletionEntry, Diagnostic, Position } from "./types";

export type Request =
	| { type: "check"; request: CheckRequest }
	| { type: "completion"; request: CompletionRequest }
	| { type: "rename"; request: RenameRequest }
	| { type: "format"; request: FormatRequest }
	| { type: "diagnostics"; request: DiangosticsRequest }
	| { type: "hover"; request: HoverRequest }
	| { type: "references"; request: ReferencesRequest }
	| { type: "definition"; request: DefinitionRequest }
	| { type: "transpile"; request: TranspileRequest };

export type Response =
	| { type: "check"; response: CheckResponse }
	| { type: "completion"; response: CompletionResponse }
	| { type: "rename"; response: RenameResponse }
	| { type: "format"; response: FormatResponse }
	| { type: "diagnostics"; response: DiagnosticsResponse }
	| { type: "hover"; response: HoverResponse }
	| { type: "references"; response: ReferencesResponse }
	| { type: "definition"; response: DefinitionResponse }
	| { type: "transpile"; response: TranspileResponse };

export type CheckRequest = { moduleIdentifiers: Array<string> };

export type CheckResponse = {
	diagnostics: { [key: string]: Array<Diagnostic> };
};

export type CompletionRequest = {
	moduleIdentifier: string;
	position: Position;
};

export type CompletionResponse = {
	entries?: Array<CompletionEntry>;
};

export type RenameRequest = {
	moduleIdentifier: string;
	position: Position;
};

export type RenameResponse = {
	locations: Array<Location> | null | undefined;
};

export type FormatRequest = {
	text: string;
};

export type FormatResponse = {
	text: string;
};

export type DiangosticsRequest = {};

export type DiagnosticsResponse = {
	diagnostics: { [key: string]: Array<Diagnostic> };
};

export type HoverRequest = {
	moduleIdentifier: string;
	position: Position;
};

export type HoverResponse = {
	text?: string;
};

export type ReferencesRequest = {
	moduleIdentifier: string;
	position: Position;
};

export type ReferencesResponse = {
	locations: Array<Location> | null;
};

export type DefinitionRequest = {
	moduleIdentifier: string;
	position: Position;
};

export type DefinitionResponse = {
	locations: Array<Location> | null;
};

export type TranspileRequest = {
	text: string;
};

export type TranspileResponse = {
	outputText: string;
	sourceMapText: string;
};
