import { Range } from "./range.ts";
import { ModuleIdentifier } from "./syscall.ts";

export type Location = {
	moduleIdentifier: ModuleIdentifier;
	range: Range;
};
