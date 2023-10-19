import * as encoding from "./encoding.ts";

export class TextEncoder {
	encode(value: string): Uint8Array {
		return encoding.utf8.encode(value);
	}
}

export class TextDecoder {
	decode(value: Uint8Array): string {
		return encoding.utf8.decode(value);
	}
}
