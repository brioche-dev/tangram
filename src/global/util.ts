export let assert: (condition: any, message?: string) => asserts condition = (
	condition,
	message,
) => {
	if (!condition) {
		message = message ?? "Failed assertion.";
		throw new Error(message);
	}
};
