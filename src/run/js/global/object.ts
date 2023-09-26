import { Blob } from "./blob.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Package } from "./package.ts";
import { Symlink } from "./symlink.ts";
import { Task } from "./task.ts";

export type Object_ =
	| { kind: "blob"; value: Blob.Object_ }
	| { kind: "directory"; value: Directory.Object_ }
	| { kind: "file"; value: File.Object_ }
	| { kind: "symlink"; value: Symlink.Object_ }
	| { kind: "package"; value: Package.Object_ }
	| { kind: "task"; value: Task.Object_ };

export namespace Object_ {
	export type Id = string;

	export class Handle {
		#state: [Id | undefined, Object_ | undefined];

		constructor(state: [Id | undefined, Object_ | undefined]) {
			this.#state = state;
		}

		state(): [Id | undefined, Object_ | undefined] {
			return this.#state;
		}

		static withId(id: Id): Handle {
			return new Handle([id, undefined]);
		}

		static withObject(object: Object_): Handle {
			return new Handle([undefined, object]);
		}

		expectId(): Id {
			if (this.#state[0] === undefined) {
				throw new Error();
			}
			return this.#state[0];
		}

		expectObject(): Object_ {
			if (this.#state[1] === undefined) {
				throw new Error();
			}
			return this.#state[1];
		}

		async id(): Promise<Id> {
			await this.store();
			return this.#state[0]!;
		}

		async object(): Promise<Object_> {
			await this.load();
			return this.#state[1]!;
		}

		async load() {
			if (this.#state[1] === undefined) {
				this.#state[1] = await syscall("load", this.#state[0]!);
			}
		}

		async store() {
			if (this.#state[0] === undefined) {
				this.#state[0] = await syscall("store", this.#state[1]!);
			}
		}
	}
}
