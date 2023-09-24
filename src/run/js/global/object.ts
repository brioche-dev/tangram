import { Blob } from "./blob.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Package } from "./package.ts";
import { Symlink } from "./symlink.ts";
import { Task } from "./task.ts";

export type Object_ =
	| Blob.Object
	| Directory.Object
	| File.Object
	| Symlink.Object
	| Package.Object
	| Task.Object;

export namespace Object_ {
	export type Id = string;

	export class Handle {
		#id: Id | undefined;
		#object: Object_ | undefined;

		static withId(id: Id): Handle {
			let handle = new Handle();
			handle.#id = id;
			return handle;
		}

		static withObject(object: Object_): Handle {
			let handle = new Handle();
			handle.#object = object;
			return handle;
		}

		expectId(): Id {
			if (this.#id === undefined) {
				throw new Error();
			}
			return this.#id;
		}

		expectObject(): Object_ {
			if (this.#object === undefined) {
				throw new Error();
			}
			return this.#object;
		}

		async id(): Promise<Id> {
			await this.store();
			return this.#id!;
		}

		async object(): Promise<Object_> {
			await this.load();
			return this.#object!;
		}

		async load() {
			if (this.#object === undefined) {
				this.#object = await syscall("load", this.#id!);
			}
		}

		async store() {
			if (this.#id === undefined) {
				this.#id = await syscall("store", this.#object!);
			}
		}
	}
}
