# Versioning

The project adheres to [Semantic Versioning](https://semver.org/), this applies
to both hardware and software, where the same version is used for both.

## Compatibility

Software remains backwards compatible with the hardware. Although some
functionality may be missing when an older version of the board is used, the
module will remain functional.

This also follows Semantic Versioning, e.g. board released under version v1.1.0
will work fine with software of v1.3.0. When compatibility is broken, the major
version will be bumped, e.g. when software requires a new button on the board,
the major version will be bumped from v1.1.0 to v2.0.0 and the software will not
work anymore with older releases of the board.

Finally, releases with major version 0 may break any time.

## Releasing

All substantial changes are recorded in the [CHANGELOG.md](CHANGELOG.md).

To release a new version of the module, versions in all `Cargo.toml` files, PCB
layout, schematic and silkscreen on the board must be adjusted. The Unreleased
section of the changelog must be renamed to the relevant version. New version of
manuals and firmware must be uploaded as artifacts to the GitHub release.

This process is automated through `./hack/release.sh <new_version>`.
