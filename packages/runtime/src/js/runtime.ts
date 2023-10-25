import { Artifact } from "./artifact.ts";
import { assert, unimplemented, unreachable } from "./assert.ts";
import { Blob, blob, download } from "./blob.ts";
import { Directory, directory } from "./directory.ts";
import * as encoding from "./encoding.ts";
import { Error as Error_, prepareStackTrace } from "./error.ts";
import { File, file } from "./file.ts";
import { include } from "./include.ts";
import { log } from "./log.ts";
import { main } from "./main.ts";
import { Args, Mutation, apply, mutation } from "./mutation.ts";
import { Object_ } from "./object.ts";
import { Package } from "./package.ts";
import { resolve } from "./resolve.ts";
import { sleep } from "./sleep.ts";
import { Symlink, symlink } from "./symlink.ts";
import { System, system } from "./system.ts";
import { Target, build, getCurrent, target } from "./target.ts";
import { Template, template } from "./template.ts";
import { Value } from "./value.ts";

Object.defineProperties(Error, {
	prepareStackTrace: { value: prepareStackTrace },
});

Object.defineProperties(globalThis, {
	console: { value: { log } },
});

async function tg(
	strings: TemplateStringsArray,
	...placeholders: Args<Template.Arg>
): Promise<Template> {
	let components: Args<Template.Arg> = [];
	for (let i = 0; i < strings.length - 1; i++) {
		let string = strings[i]!;
		components.push(string);
		let placeholder = placeholders[i]!;
		components.push(placeholder);
	}
	components.push(strings[strings.length - 1]!);
	return await template(...components);
}

Object.assign(tg, {
	Artifact,
	Blob,
	Directory,
	Error: Error_,
	File,
	Mutation,
	Object_,
	Package,
	Symlink,
	System,
	Target,
	Template,
	Value,
	apply,
	assert,
	blob,
	build,
	directory,
	download,
	encoding,
	file,
	include,
	log,
	main,
	mutation,
	resolve,
	sleep,
	symlink,
	system,
	target,
	template,
	unimplemented,
	unreachable,
});

Object.defineProperties(tg, {
	current: { get: getCurrent },
});

Object.defineProperties(globalThis, {
	tg: { value: tg },
});
