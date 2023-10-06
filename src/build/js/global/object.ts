import { Blob } from "./blob.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Package } from "./package.ts";
import { Symlink } from "./symlink.ts";
import { Target } from "./target.ts";

export type Kind =
	| "blob"
	| "directory"
	| "file"
	| "symlink"
	| "package"
	| "target";

export type Object_ =
	| { kind: "blob"; value: Blob.Object_ }
	| { kind: "directory"; value: Directory.Object_ }
	| { kind: "file"; value: File.Object_ }
	| { kind: "symlink"; value: Symlink.Object_ }
	| { kind: "package"; value: Package.Object_ }
	| { kind: "target"; value: Target.Object_ };

export namespace Object_ {
	export type Id = string;

	export class Handle {
		#state: State;

		constructor(state: State) {
			this.#state = state;
		}

		state(): State {
			return this.#state;
		}

		static withId(id: Id): Handle {
			return new Handle({ id, object: undefined });
		}

		static withObject(object: Object_): Handle {
			return new Handle({ id: undefined, object });
		}

		expectId(): Id {
			if (this.#state.id === undefined) {
				throw new Error();
			}
			return this.#state.id;
		}

		expectObject(): Object_ {
			if (this.#state.object === undefined) {
				throw new Error();
			}
			return this.#state.object;
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
