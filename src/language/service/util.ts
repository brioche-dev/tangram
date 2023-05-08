import { TSESTree, visitorKeys } from "@typescript-eslint/typescript-estree";

export let assert: (
	condition: unknown,
	message?: string,
) => asserts condition = (condition, message) => {
	if (!condition) {
		message = message ?? "Failed assertion.";
		throw new Error(message);
	}
};

/** Visit each node in a tree. */
export let visit = (
	node: TSESTree.Node,
	visitor: (node: TSESTree.Node) => void,
) => {
	// Visit the root.
	visitor(node);

	// Visit the children.
	let keys = visitorKeys[node.type];
	if (keys) {
		for (let key of keys) {
			let child: TSESTree.Node | Array<TSESTree.Node> | undefined = (
				node as any
			)[key];
			if (child instanceof Array) {
				for (let item of child) {
					visit(item, visitor);
				}
			} else if (child) {
				visit(child, visitor);
			}
		}
	}
};
