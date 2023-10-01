export type Module =
	| { kind: "document"; value: DocumentModule }
	| { kind: "library"; value: LibraryModule }
	| { kind: "normal"; value: NormalModule };

export type DocumentModule = {
	packagePath: string;
	modulePath: string;
};

export type LibraryModule = {
	modulePath: string;
};

export type NormalModule = {
	packageId: string;
	modulePath: string;
};
