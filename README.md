<div align="center">
    <h1>Tarmac</h1>
</div>

<div align="center">
    <a href="https://github.com/Vorlias/tarmac/actions">
        <img src="https://github.com/Vorlias/tarmac/workflows/CI/badge.svg" alt="GitHub Actions status" />
    </a>
</div>

<hr />

Tarmac is inspired by projects like [Webpack](https://webpack.js.org/) that make it easy to reference assets from code.

## Installation

### Installing with Aftman
The recommended way to install Tarmac is with [Aftman](https://github.com/LPGhatguy/aftman).

Add an entry to the `[tools]` section of your `aftman.toml` file:

```toml
tarmac = "rojo-rbx/tarmac@0.7.4"
```

### Installing with Foreman

Add an entry to the `[tools]` section of your `foreman.toml` file:

```toml
tarmac = { source = "rojo-rbx/tarmac", version = "0.7.4" }
```


### Installing from GitHub Releases
Pre-built binaries are available for 64-bit Windows, macOS, and Linux from the [GitHub releases page](https://github.com/Roblox/tarmac/releases).

## Basic Example
**The [examples](examples) folder contains small, working projects using different features from Tarmac.**

Tarmac is configured by a [TOML](https://github.com/toml-lang/toml) file in the root of a project named `tarmac.toml`. Tarmac uses this file to determine where to look for assets and what to do with them.

To tell Tarmac to manage PNG files in a folder named `assets`, you can use:

```toml
name = "basic-tarmac-example"

# Most projects will define some 'inputs'.
# This tells Tarmac where to find assets that we'll use in our game.
[[inputs]]
glob = "assets/**/*.png"
codegen = true
codegen-path = "src/assets.lua"
codegen-base-path = "assets"
```

Run `tarmac sync --target roblox` to have Tarmac upload any new or updated assets that your project depends on. You may need to pass a `.ROBLOSECURITY` cookie explicitly via the `--auth` argument.

Tarmac will generate Lua code in `src/assets.lua` that looks something like this:

```lua
-- This file was @generated by Tarmac. It is not intended for manual editing.
return {
	foo = {
		bar = "rbxassetid://238549023",
		baz = "rbxassetid://238549024",
	}
}
```

These files will be turned into `ModuleScript` instances by tools like [Rojo](https://github.com/rojo-rbx/rojo). From there, it's easy to load this module and reference the assets within:

```lua
local assets = require(script.Parent.assets)

local decal = Instance.new("Decal")
decal.Texture = assets.foo.bar
```

## Command Line Interface
For more information, run `tarmac --help`.

### Global Options
These options can be specified alongside any subcommands and are all optional.

* `--help`, `-h`
	* Prints help information about Tarmac and exits.
* `--version`, `-V`
	* Prints version information about Tarmac and exits.
* `--api-key <key>`
	* Defines the API key Tarmac will use to authenticate with Open Cloud.
	* If not specified, Tarmac will fall back to the cookie authentication method.
* `--auth <cookie>`
	* Explicitly defines the authentication cookie Tarmac should use to communicate with Roblox.
	* If not specified, Tarmac will attempt to locate one from the local system.
* `--verbose`, `-v`
	* Enables more verbose logging. Can be specified up to three times, which will increase verbosity further.

### `tarmac sync`
Detects changes to assets in the local project and attempts to synchronize them with an external service, like the Roblox cloud.

Usage:
```bash
tarmac sync [<config-path>] \
	--target <roblox|debug|none>
	--retry <number>
	--retry-delay <60>
```

To sync the project in your current working directory with the Roblox cloud, use:
```bash
tarmac sync --target roblox
```

To validate that all inputs are already synced, use the `none` target:
```bash
tarmac sync --target none
```

When tarmac gets rate limited while syncing to Roblox, use the `--retry` argument to automatically attempt to re-upload. This will tell tarmac how many times it can attempt to re-upload each asset. The `--retry-delay` sets the number of seconds to wait between each attempt.
```bash
tarmac sync --target roblox --retry 3
```

### `tarmac upload-image`
Uploads a single image as a decal and prints the ID of the resulting image asset to stdout.

Usage:
```bash
tarmac upload-image <image-path> \
	--name <asset-name> \
	--description <asset-description>
```

Example:
```bash
tarmac upload-image foo.png --name "Foo" --description "Foo is a placeholder name."
```

### `tarmac asset-list`
Outputs a list of all of the asset IDs referenced by the project. Each ID is separated by a newline.

Usage:
```bash
tarmac asset-list [<config-path>] \
	--output <file-path>
```

Example:
```bash
tarmac asset-list --output asset-list.txt
```

### `tarmac create-cache-map`
Creates a mapping from asset IDs back to their source files. Also downloads packaged images to a given folder, generating links to those assets as well.

The mapping file is JSON.

Usage:
```bash
tarmac create-cache-map [<config-path>] \
	--index-file <file-path> \
	--cache-dir <cache-folder>
```

Example:
```bash
tarmac create-cache-map --index-file assets.json --cache-dir asset-cache
```

### `tarmac help`
Prints help information about Tarmac itself, or the given subcommand.

Usage:
```bash
tarmac help [<subcommand>]
```

## Project Format
* `name`, string
	* The name of the Tarmac project, used in logging and error reporting.
* `max-spritesheet-size`, (int, int), **optional**
	* The maximum spritesheet size that Tarmac should use. Defaults to **(1024, 1024)**, the maximum image size supported by Roblox.
* `asset-cache-path`, path, **optional**
	* If defined, Tarmac will re-download uploaded images to a local folder at the given path. Files in this folder not associated with assets in the project will be deleted.
* `asset-list-path`, path, **optional**
	* If defined, Tarmac will write a list of asset URLs used by the project to the given file. One URL is printed per line.
* `upload-to-group-id`, int, **optional**
	* If defined, Tarmac will attempt to upload all assets to the given Roblox Group. If unable, syncing will fail.
* `upload-to-user-id`, int, **optional**
	* If defined, Tarmac will attempt to upload assets to the given Roblox user. This option is required when using the Open Cloud API via `--api-key`, but has no effect when using cookie authentication.
* `inputs`, list\<InputConfig\>, **optional**
	* A list of inputs that Tarmac will process.
* `includes`, list\<path\>, **optional**
	* A list of additional paths to search recursively for additional projects in. The inputs from discovered projects will be merged into this project, and other settings ignored.
	* When a `tarmac.toml` file is found, Tarmac will include it and its includes and stop traversing that directory.

### InputConfig
* `glob`, string
	* A path glob that should include any files for this input group.
	* Tarmac uses the [globset library](https://docs.rs/globset/0.4.5/globset/) and supports any syntax it supports.
* `codegen`, bool, **optional**
	* Whether Tarmac should generate Lua code for the assets contained in this input group. Defaults to **false**.
* `codegen-path`, path, **optional**
	* If defined and `codegen` is true, Tarmac will merge all generated Lua code for this input group into a single file.
* `codegen-base-path`, path, **optional**
	* Defines the base path for generating Lua code when `codegen-path` is also defined. Defaults to **the directory containing `tarmac.toml`**.

## License
Tarmac is available under the MIT license. See [LICENSE.txt](LICENSE.txt) for details.
