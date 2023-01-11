import { Location, CompletionEntry, Diagnostic, Position } from "./types";

export type Request =
	| { type: "check"; request: CheckRequest }
	| { type: "completion"; request: CompletionRequest }
	| { type: "find_rename_locations"; request: FindRenameLocationsRequest }
	| { type: "format"; request: FormatRequest }
	| { type: "get_diagnostics"; request: GetDiangosticsRequest }
	| { type: "get_hover"; request: GetHoverRequest }
	| { type: "get_references"; request: GetReferencesRequest }
	| { type: "goto_definition"; request: GotoDefinitionRequest }
	| { type: "transpile"; request: TranspileRequest };

export type Response =
	| { type: "check"; response: CheckResponse }
	| { type: "completion"; response: CompletionResponse }
	| { type: "find_rename_locations"; response: FindRenameLocationsResponse }
	| { type: "format"; response: FormatResponse }
	| { type: "get_diagnostics"; response: GetDiagnosticsResponse }
	| { type: "get_hover"; response: GetHoverResponse }
	| { type: "get_references"; response: GetReferencesResponse }
	| { type: "goto_definition"; response: GotoDefinitionResponse }
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

export type FindRenameLocationsRequest = {
	moduleIdentifier: string;
	position: Position;
};

export type FindRenameLocationsResponse = {};

export type FormatRequest = {
	text: string;
};

export type FormatResponse = {
	text: string;
};

export type GetDiangosticsRequest = {};

export type GetDiagnosticsResponse = {
	diagnostics: { [key: string]: Array<Diagnostic> };
};

export type GetHoverRequest = {
	moduleIdentifier: string;
	position: Position;
};

export type GetHoverResponse = {
	text?: string;
};

export type GetReferencesRequest = {
	moduleIdentifier: string;
	position: Position;
};

export type GetReferencesResponse = {
	locations: Array<Location> | null;
};

export type GotoDefinitionRequest = {
	moduleIdentifier: string;
	position: Position;
};

export type GotoDefinitionResponse = {
	locations: Array<Location> | null;
};

export type TranspileRequest = {
	text: string;
};

export type TranspileResponse = {
	outputText: string;
	sourceMapText: string;
};
