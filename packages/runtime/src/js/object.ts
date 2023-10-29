import { Branch } from "./branch.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Leaf } from "./leaf.ts";
import { Package } from "./package.ts";
import { Symlink } from "./symlink.ts";
import { Target } from "./target.ts";

export type Object_ =
	| { kind: "leaf"; value: Leaf.Object_ }
	| { kind: "branch"; value: Branch.Object_ }
	| { kind: "directory"; value: Directory.Object_ }
	| { kind: "file"; value: File.Object_ }
	| { kind: "symlink"; value: Symlink.Object_ }
	| { kind: "package"; value: Package.Object_ }
	| { kind: "target"; value: Target.Object_ };

export namespace Object_ {
	export type Id = string;

	export type Kind =
		| "leaf"
		| "branch"
		| "directory"
		| "file"
		| "symlink"
		| "package"
		| "target";

	export class Handle {
		#state: State;

		constructor(state: State) {
			this.#state = state;
		}

		get state(): State {
			return this.#state;
		}

		static withId(id: Id): Handle {
			return new Handle({ id, object: undefined });
		}

		static withObject(object: Object_): Handle {
			return new Handle({ id: undefined, object });
		}

		async id(): Promise<Id> {
			await this.store();
			return this.#state.id!;
		}

		async object(): Promise<Object_> {
			await this.load();
			return this.#state.object!;
		}

		async load() {
			if (this.#state.object === undefined) {
				this.#state.object = await syscall("load", this.#state.id!);
			}
		}

		async store() {
			if (this.#state.id === undefined) {
				this.#state.id = await syscall("store", this.#state.object!);
			}
		}
	}

	export type State = {
		id: Id | undefined;
		object: Object_ | undefined;
	};
}
