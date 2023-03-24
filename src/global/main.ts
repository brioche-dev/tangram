import { array } from "./array";
import { isArtifact } from "./artifact";
import { checksum } from "./checksum";
import { context } from "./context";
import { Directory, directory, isDirectory } from "./directory";
import { download } from "./download";
import { prepareStackTrace } from "./error";
import { File, file, isFile } from "./file";
import { function_ } from "./function";
import { include } from "./include";
import { log } from "./log";
import { map } from "./map";
import { Path, isPath, path } from "./path";
import { Placeholder, isPlaceholder, placeholder } from "./placeholder";
import { output, process } from "./process";
import { Reference, isReference, reference } from "./reference";
import { resolve } from "./resolve";
import { Symlink, isSymlink, symlink } from "./symlink";
import { Template, isTemplate, t, template } from "./template";

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
	Directory,
	File,
	Path,
	Placeholder,
	Reference,
	Symlink,
	Template,
	array,
	checksum,
	context,
	directory,
	download,
	file,
	function: function_,
	include,
	isArtifact,
	isDirectory,
	isFile,
	isPath,
	isPlaceholder,
	isReference,
	isSymlink,
	isTemplate,
	log,
	map,
	output,
	path,
	placeholder,
	process,
	reference,
	resolve,
	symlink,
	template,
};
Object.defineProperties(globalThis, {
	t: { value: t },
	tg: { value: tg },
});
