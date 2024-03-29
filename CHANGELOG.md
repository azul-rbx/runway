# Tarmac Changelog

## Unreleased Changes
* Added `spritesheet-padding-size` to root config ([#11](https://github.com/rojo-rbx/tarmac/pull/11))

## 0.7.1 (2020-12-14)
* Added alpha-bleeding of unpacked images ([#2](https://github.com/jeparlefrancais/tarmac/pull/2))
* Added option to sync files locally ([#1](https://github.com/jeparlefrancais/tarmac/pull/1))

## 0.7.0 (2020-07-23)
* Added support for automatically retrying uploads when being rate limited. ([#42](https://github.com/Roblox/tarmac/issues/43))

## 0.6.0 (2020-07-09)
* Added support for automatically retrying image uploads when image names are moderated. ([#26](https://github.com/Roblox/tarmac/issues/26))
* Added `create-cache-map` subcommand to aid in prepopulating asset caches. ([#38](https://github.com/Roblox/tarmac/pull/38))
* Added `asset-list` subcommand to generate a plaintext list of referenced asset IDs.

## 0.5.0 (2020-06-01)
* **Breaking**: Codegen style is no longer configurable. The correct codegen style is now chosen based on the kind of input. ([#28](https://github.com/Roblox/tarmac/pull/28))
* **Breaking**: `includes` is now a list of paths instead of a list of objects.
* **Breaking**: Renamed `base-path` to `codegen-base-path` to better reflect its purpose.
* Removed (unimplemented) `content-folder` target and added `none` target.
	* This target always fails to upload, and is useful to verify that all assets have been uploaded as part of a CI job.
* Added `asset-cache-path` config option. If specified, Tarmac will download managed assets from Roblox.com to populate the given directory with.
* Added `upload-to-group-id` config option to require that all uploaded assets are uploaded to the given group.
* Added `asset-list-path` for generating a list of all asset URLs referred to by the Tarmac project.
	* This output format is intended for consumption by other tools.
* Fixed handling of HTTP 429 (Too Many Requests) responses from the Roblox asset endpoints.
	* Tarmac will now save its progress and exit with an error in this case.

## 0.4.0 (2020-03-04)
* Tarmac now "alpha-bleeds" packed image spritesheets to prevent artifacts from appearing when resized in Roblox.
* Reworked Tarmac's codegen strategy. ([#22](https://github.com/Roblox/tarmac/pull/22)
	* Inputs can now specify `codegen-path` and `base-path` to group together modules.
	* This helps reduce diff noise drastically.

## 0.3.1 (2020-02-04)
* Fixed `tarmac sync` sometimes re-uploading more images than it should. ([#19](https://github.com/Roblox/tarmac/pull/19))
* Updated `tarmac-manifest.toml` to require hashes. This might cause errors when upgrading to 0.3.1; they can be fixed by deleting your manifest and syncing again.

## 0.3.0 (2020-01-31)
* Rewrote texture packing routine with a new library, [Packos](https://crates.io/crates/packos).
	* This should fix textures overlapping eachother when running Tarmac with automatic spritesheets enabled.

## 0.2.0 (2020-01-21)
* Revamped configuration format.
* Added support for automatically packing spritesheets.
* Added support for nesting projects inside eachother via `include`.
* Added support for grabbing inputs by glob pattern.

## 0.1.0 (2020-01-03)
* Initial release.
