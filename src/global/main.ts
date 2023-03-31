import { array } from "./array.ts";
import { isArtifact } from "./artifact.ts";
import { Blob, blob, isBlobLike } from "./blob.ts";
import { bundle } from "./bundle.ts";
import { checksum } from "./checksum.ts";
import { context } from "./context.ts";
import { Directory, directory, isDirectory } from "./directory.ts";
import { download } from "./download.ts";
import { prepareStackTrace } from "./error.ts";
import { File, file, isFile } from "./file.ts";
import { function_ } from "./function.ts";
import { include } from "./include.ts";
import { log } from "./log.ts";
import { map } from "./map.ts";
import { Path, isPath, path } from "./path.ts";
import { Placeholder, isPlaceholder, placeholder } from "./placeholder.ts";
import { output, process } from "./process.ts";
import { resolve } from "./resolve.ts";
import { Symlink, isSymlink, symlink } from "./symlink.ts";
import { Template, isTemplate, t, template } from "./template.ts";

// Set `Error.prepareStackTrace`.
Object.defineProperties(Error, {
	prepareStackTrace: { value: prepareStackTrace },
});

// Create the console global.
let console = {
	log,
};
Object.defineProperties(globalThis, {
	console: { value: console },
});

// Create the tg global.
let tg = {
	Blob,
	Directory,
	File,
	Path,
	Placeholder,
	Symlink,
	Template,
	array,
	blob,
	bundle,
	checksum,
	context,
	directory,
	download,
	file,
	function: function_,
	include,
	isArtifact,
	isBlobLike,
	isDirectory,
	isFile,
	isPath,
	isPlaceholder,
	isSymlink,
	isTemplate,
	log,
	map,
	output,
	path,
	placeholder,
	process,
	resolve,
	symlink,
	template,
};
Object.defineProperties(globalThis, {
	t: { value: t },
	tg: { value: tg },
});
