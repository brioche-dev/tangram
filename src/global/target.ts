import "./syscall";
import { run } from "./operation";
import { Package, PackageHash, addPackage, getPackage } from "./package";
import { Unresolved, resolve } from "./resolve";
import { MaybePromise } from "./util";
import { serializeValue, deserializeValue, Value } from "./value";

type TargetArgs = {
	package: Package;
	name: string;
	args?: Array<Value> | null | undefined;
};

export let target = async <T extends Value>(args: TargetArgs): Promise<T> => {
	return await new Target<T>(args).run();
};

export class Target<T extends Value = Value> {
	package: Package;
	name: string;
	args: Array<Value> | null | undefined;

	constructor(args: TargetArgs) {
		this.package = args.package;
		this.name = args.name;
		this.args = args.args;
	}

	async serialize(): Promise<syscall.Target> {
		let package_ = await addPackage(this.package);
		let name = this.name;
		let args = this.args
			? await Promise.all(this.args.map((arg) => serializeValue(arg)))
			: null;
		return {
			package: package_.toString(),
			name,
			args,
		};
	}

	static async deserialize<T extends Value>(
		target: syscall.Target,
	): Promise<Target<T>> {
		let package_ = await getPackage(new PackageHash(target.package));
		let name = target.name;
		let args = target.args
			? await Promise.all(target.args.map((arg) => deserializeValue(arg)))
			: null;
		return new Target({
			package: package_,
			name,
			args,
		});
	}

	async run(): Promise<T> {
		return await run(this);
	}
}

export type TargetFunction<A extends Value, R extends Value> = {
	(args: Unresolved<A>): MaybePromise<R>;
	run?: (args: syscall.Value) => Promise<syscall.Value>;
};

export let createTarget = <A extends Value, R extends Value>(
	f: (args: A) => MaybePromise<R>,
): TargetFunction<A, R> => {
	// Get the target's package and name.
	let packageHash = new PackageHash(syscall("get_current_package_hash"));
	let name = syscall("get_target_name");

	// Create the target function.
	let targetFunction: TargetFunction<A, R> = async (args: Unresolved<A>) => {
		let resolvedArgs = await resolve(args);
		return await target({
			package: await getPackage(packageHash),
			name,
			args: [resolvedArgs],
		});
	};

	// Create the target function's run method.
	targetFunction.run = async (serializedArgs: syscall.Value) => {
		let args = (await deserializeValue(serializedArgs)) as A;
		let output = await f(args);
		let serializedOutput = await serializeValue(output);
		return serializedOutput;
	};

	return targetFunction;
};
