import { Artifact } from "./artifact.ts";
import { Block } from "./block.ts";

export let package_ = () => {};

export class Package {
	#block!: Block;
	#artifact!: Artifact;
	#dependencies!: { [dependency: string]: Block } | undefined;
}
