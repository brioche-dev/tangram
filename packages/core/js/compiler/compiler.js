globalThis.handle = (request) => {
	switch (request.type) {
		case "check": {
			return {
				type: "check",
				content: {
					diagnostics: [`It worked!`],
				},
			};
		}
		default: {
			throw new Error("Unknown request type.");
		}
	}
};
