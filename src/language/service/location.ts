import { Range } from "./range.ts";
import { Module } from "./syscall.ts";

export type Location = {
	module: Module;
	range: Range;
};
