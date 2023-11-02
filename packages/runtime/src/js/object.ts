import { Branch } from "./branch.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Leaf } from "./leaf.ts";
import { Lock } from "./lock.ts";
import { Symlink } from "./symlink.ts";
import { Target } from "./target.ts";

export type Object_ =
	| { kind: "leaf"; value: Leaf.Object_ }
	| { kind: "branch"; value: Branch.Object_ }
	| { kind: "directory"; value: Directory.Object_ }
	| { kind: "file"; value: File.Object_ }
	| { kind: "symlink"; value: Symlink.Object_ }
	| { kind: "lock"; value: Lock.Object_ }
	| { kind: "target"; value: Target.Object_ };

export namespace Object_ {
	export type Id = string;

	export type Kind =
		| "leaf"
		| "branch"
		| "directory"
		| "file"
		| "symlink"
		| "lock"
		| "target";

	export type State<I, O> = {
		id?: I | undefined;
		object?: O | undefined;
	};
}
