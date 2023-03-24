import { Range } from "./range";
import { ModuleIdentifier } from "./syscall";

export type Location = {
	moduleIdentifier: ModuleIdentifier;
	range: Range;
};
