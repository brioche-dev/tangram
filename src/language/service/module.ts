export type Module =
	| { kind: "library"; value: LibraryModule }
	| { kind: "document"; value: DocumentModule }
	| { kind: "normal"; value: NormalModule };

export type LibraryModule = {
	modulePath: string;
};

export type DocumentModule = {
	packagePath: string;
	modulePath: string;
};

export type NormalModule = {
	package: string;
	modulePath: string;
};
