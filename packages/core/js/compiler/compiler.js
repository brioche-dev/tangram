globalThis.handle = async (request) => {
	switch (request.type) {
		// Handle a check request.
		case "check": {
			return {
				type: "check",
				content: {
					diagnostics: [Deno.core.opSync("op_example")],
				},
			};
		}

		default: {
			throw new Error("Unknown request type.");
		}
	}
};
