import { TangramLanguageClient } from "./language_client";
import * as vscode from "vscode";
import * as vscodeLanguageClient from "vscode-languageclient";

export interface VirtualTextDocumentParams {
	textDocument: vscodeLanguageClient.TextDocumentIdentifier;
}

let virtualTextDocument = new vscodeLanguageClient.RequestType<
	VirtualTextDocumentParams,
	string,
	void
>("tangram/virtualTextDocument");

export class TangramTextDocumentContentProvider
	implements vscode.TextDocumentContentProvider
{
	constructor(private client: TangramLanguageClient) {}

	provideTextDocumentContent(
		uri: vscode.Uri,
		token: vscode.CancellationToken,
	): vscode.ProviderResult<string> {
		if (!this.client.languageClient) {
			throw new Error("Tangram language server has not started.");
		}

		return this.client.languageClient.sendRequest(
			virtualTextDocument,
			{ textDocument: { uri: uri.toString() } },
			token,
		);
	}
}
