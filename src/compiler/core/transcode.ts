export let encodeHex = (bytes: Uint8Array): string => {
	let hexString = "";
	for (let i = 0; i < bytes.length; i++) {
		hexString += bytes[i].toString(16);
	}
	return hexString;
};

let hexRegex = /^([0-9a-fA-F]{2})*$/;
export let decodeHex = (hexString: string): Uint8Array => {
	if (!hexRegex.test(hexString)) {
		throw new Error(`Invalid hex string: ${hexString}`);
	}
	let bytesLength = hexString.length / 2;
	let bytes = new Uint8Array(bytesLength);
	for (let i = 0; i < bytesLength; i++) {
		let hexOffset = i * 2;
		let hexByte = hexString.slice(hexOffset, hexOffset + 2);
		bytes[i] = parseInt(hexByte, 16);
	}
	return bytes;
};

/**
 * Convert a `Uint8Array` to a base64-encoded string.
 *
 * @remarks
 *
 * Based on the base64 conversion methods from MDN: {@link https://developer.mozilla.org/en-US/docs/Glossary/Base64}.
 */
export let encodeBase64 = (aBytes: Uint8Array): string => {
	let nMod3 = 2;
	let sB64Enc = "";
	let uint6ToB64 = (nUint6: number): number => {
		return nUint6 < 26
			? nUint6 + 65
			: nUint6 < 52
			? nUint6 + 71
			: nUint6 < 62
			? nUint6 - 4
			: nUint6 === 62
			? 43
			: nUint6 === 63
			? 47
			: 65;
	};
	let nLen = aBytes.length;
	let nUint24 = 0;
	for (let nIdx = 0; nIdx < nLen; nIdx++) {
		nMod3 = nIdx % 3;
		if (nIdx > 0 && ((nIdx * 4) / 3) % 76 === 0) {
			sB64Enc += "\r\n";
		}
		nUint24 |= aBytes[nIdx] << ((16 >>> nMod3) & 24);
		if (nMod3 === 2 || aBytes.length - nIdx === 1) {
			sB64Enc += String.fromCodePoint(
				uint6ToB64((nUint24 >>> 18) & 63),
				uint6ToB64((nUint24 >>> 12) & 63),
				uint6ToB64((nUint24 >>> 6) & 63),
				uint6ToB64(nUint24 & 63),
			);
			nUint24 = 0;
		}
	}
	return (
		sB64Enc.slice(0, sB64Enc.length - 2 + nMod3) +
		(nMod3 === 2 ? "" : nMod3 === 1 ? "=" : "==")
	);
};

/**
 * Convert a base64-encoded string to a `Uint8Array`.
 *
 * @remarks
 *
 * Based on the base64 conversion methods from MDN: {@link https://developer.mozilla.org/en-US/docs/Glossary/Base64}.
 */
export let decodeBase64 = (
	sBase64: string,
	nBlocksSize: number = 1,
): Uint8Array => {
	let sB64Enc = sBase64.replace(/[^A-Za-z0-9+/]/g, "");
	let nInLen = sB64Enc.length;
	let nOutLen = Math.ceil(((nInLen * 3 + 1) >> 2) / nBlocksSize) * nBlocksSize;
	let taBytes = new Uint8Array(nOutLen);
	let b64ToUint6 = (nChr: number): number => {
		return nChr > 64 && nChr < 91
			? nChr - 65
			: nChr > 96 && nChr < 123
			? nChr - 71
			: nChr > 47 && nChr < 58
			? nChr + 4
			: nChr === 43
			? 62
			: nChr === 47
			? 63
			: 0;
	};
	let nMod3: number;
	let nMod4: number;
	let nUint24 = 0;
	let nOutIdx = 0;
	for (let nInIdx = 0; nInIdx < nInLen; nInIdx++) {
		nMod4 = nInIdx & 3;
		nUint24 |= b64ToUint6(sB64Enc.charCodeAt(nInIdx)) << (6 * (3 - nMod4));
		if (nMod4 === 3 || nInLen - nInIdx === 1) {
			nMod3 = 0;
			while (nMod3 < 3 && nOutIdx < nOutLen) {
				taBytes[nOutIdx] = (nUint24 >>> ((16 >>> nMod3) & 24)) & 255;
				nMod3++;
				nOutIdx++;
			}
			nUint24 = 0;
		}
	}
	return taBytes;
};

export let encodeUtf8 = (string: string): Uint8Array => {
	let bytes: Array<number> = [];
	let length = string.length;
	let i = 0;
	while (i < length) {
		let codePoint = string.codePointAt(i);
		if (codePoint === undefined) throw new Error("Invalid code point.");
		let c = 0;
		let bits = 0;
		if (codePoint <= 0x0000007f) {
			c = 0;
			bits = 0x00;
		} else if (codePoint <= 0x000007ff) {
			c = 6;
			bits = 0xc0;
		} else if (codePoint <= 0x0000ffff) {
			c = 12;
			bits = 0xe0;
		} else if (codePoint <= 0x001fffff) {
			c = 18;
			bits = 0xf0;
		}
		bytes.push(bits | (codePoint >> c));
		c -= 6;
		while (c >= 0) {
			bytes.push(0x80 | ((codePoint >> c) & 0x3f));
			c -= 6;
		}
		i += codePoint >= 0x10000 ? 2 : 1;
	}
	return new Uint8Array(bytes);
};

export let decodeUtf8 = (bytes: Uint8Array): string => {
	var string = "";
	var i = 0;
	while (i < bytes.length) {
		var octet = bytes[i];
		var bytesNeeded = 0;
		var codePoint = 0;
		if (octet <= 0x7f) {
			bytesNeeded = 0;
			codePoint = octet & 0xff;
		} else if (octet <= 0xdf) {
			bytesNeeded = 1;
			codePoint = octet & 0x1f;
		} else if (octet <= 0xef) {
			bytesNeeded = 2;
			codePoint = octet & 0x0f;
		} else if (octet <= 0xf4) {
			bytesNeeded = 3;
			codePoint = octet & 0x07;
		}
		if (bytes.length - i - bytesNeeded > 0) {
			var k = 0;
			while (k < bytesNeeded) {
				octet = bytes[i + k + 1];
				codePoint = (codePoint << 6) | (octet & 0x3f);
				k += 1;
			}
		} else {
			codePoint = 0xfffd;
			bytesNeeded = bytes.length - i;
		}
		string += String.fromCodePoint(codePoint);
		i += bytesNeeded + 1;
	}
	return string;
};
