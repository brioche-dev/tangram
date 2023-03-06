import { isArtifact } from "./artifact";
import { context } from "./context";
import { Directory, directory, isDirectory } from "./directory";
import { download } from "./download";
import { File, file, isFile } from "./file";
import { function_ } from "./function";
import { log } from "./log";
import { Path, isPath, path } from "./path";
import { Placeholder, isPlaceholder, placeholder } from "./placeholder";
import { output, process } from "./process";
import { Reference, isReference, reference } from "./reference";
import { resolve } from "./resolve";
import { Symlink, isSymlink, symlink } from "./symlink";
import { Template, isTemplate, t, template } from "./template";

let console = {
	log,
};

let tg = {
	Directory,
	File,
	Path,
	Placeholder,
	Reference,
	Symlink,
	Template,
	context,
	directory,
	download,
	file,
	function: function_,
	isArtifact,
	isDirectory,
	isFile,
	isPath,
	isPlaceholder,
	isReference,
	isSymlink,
	isTemplate,
	log,
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
	console: { value: console },
	t: { value: t },
	tg: { value: tg },
});
