<p align="center">
	<img width="200px" src="tangram.svg" title="Tangram">
</p>

# Tangram

Tangram is a programmable build system and package manager in which all dependencies are specified explicitly and pinned with a lockfile. You get the exact same versions of every package on every machine, so your builds are **simple**, **reproducible**, **cacheable**, and **distributable**.

- **Simple.** Write your builds in JSON, JavaScript, or TypeScript.
- **Reproducible.** Stop debugging errors caused by inconsistent package versions.
- **Cacheable.** Stop building the same thing over and over again.
- **Distributable.** Transparently offload your builds to a cluster or the cloud.

## Install.

Install Tangram on macOS or Linux with the command below.

```
curl https://install.tangram.dev | sh
```

## Examples

- [Build a shell.](#build-a-shell)
- [Build your code.](#build-your-code)
- [Build a container.](#build-a-container)
- [Pin and patch dependencies.](#pin-and-patch-dependencies)
- [Try software without modifying your system.](#try-software-without-modifying-your-system)
- [Build and run on virtual machines.](#build-and-run-on-virtual-machines)

### Build a shell.

Create a `tangram.json` file at the root of your project and add some dependencies.

```json
{
	"dependencies": {
		"nodejs": "16.15.1",
		"postgresql": "14.4",
		"python": "3.10.4",
		"ripgrep": "13.0.0"
	}
}
```

Now run `tg shell`. This will create a `tangram.lock` file and drop you in a shell in which your dependencies appear in `$PATH`.

```
$ tg shell
$ node --version
v16.15.1
$ postgres --version
postgres (PostgreSQL) 14.4
$ python --version
Python 3.10.4
$ rg --version
ripgrep 13.0.0
```

This is a great way to make sure everyone on your team is using the exact same versions of all the tools you need to work on your project.

For more convenience but with less isolation, you can run `source $(tg shell --source)` to set the environment variables in your current shell. You can also run `tg autoshell enable` to enable an autoshell, which will automatically set the environment variables whenever you `cd` into your project.

### Build your code.

Use Tangram to build both your dependencies and your code. In this example, we build a Rust project and specify the exact version of the OpenSSL C library to link to.

```javascript

```

### Build a container image.

Use the `std.buildContainerImage` function to build a container image. In this example, we build a container image with a simple python project.

```javascript

```

Container image builds with Tangram are **fast**, **reproducible**, and **minimal**.

- **Fast.** Tangram builds form a graph, not a line, so you don't have to rebuild anything that did not change.
- **Reproducible.** You get the exact same versions of every package on every build.
- **Minimal.** There is no base image. Your container image contains only the packages you specify.

## Pin and patch dependencies.

Tangram packages come with a lot of options for customization. In this example, we build the Zig compiler at a particular revision from GitHub and apply a patch from an unmerged pull request. Now every machine that uses this shell will have the same custom build of Zig.

```js

```

## Try software without modifying your system.

Everything you build with Tangram is self contained and isolated from the rest of your system, so you can try software without affecting your other projects.

```
$ tg run ripgrep@13.0.0 -- --version
ripgrep 13.0.0
```

## Build and run on virtual machines.

Tangram has built-in virtualization so you can run builds and shells for other architectures and operating systems. Try the command below on macOS.

```
$ tg shell --system x86_64-linux -p coreutils "uname -sm"
Linux x86_64
```

<!--
## Learn more.

To learn more about Tangram and how it works, please [read the blog post](https://www.tangram.dev/blog/hello_world).
-->
