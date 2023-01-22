#!/usr/bin/env bash
set -euo pipefail

version=${1}

sed -i "s/## Unreleased/## Unreleased\n\n## ${version}/" CHANGELOG.md
sed -i "s/version =.* # hack\/release.sh$/version = \"${version}\" # hack\/release.sh/" firmware/Cargo.toml
sed -i "s/version =.* # hack\/release.sh$/version = \"${version}\" # hack\/release.sh/" control/Cargo.toml
sed -i "s/version =.* # hack\/release.sh$/version = \"${version}\" # hack\/release.sh/" dsp/Cargo.toml
sed -i "s/version =.* # hack\/release.sh$/version = \"${version}\" # hack\/release.sh/" benches/Cargo.toml
sed -i "s/rev .*/rev \"v${version}\")/" hardware/Module.kicad_sch
sed -i "s/gr_text \"board .*\" /gr_text \"board v${version}\" /" hardware/Module.kicad_pcb
sed -i "s/rev .*/rev \"v${version}\")/" hardware/Module.kicad_pcb

makers

rm -rf release
mkdir release

pushd eurorack && cargo objcopy --release -- -O binary ../release/kaseta-firmware-${version}.bin && popd

make manual
cp manual/user/manual.pdf release/kaseta-user-manual.pdf
# cp manual/build/manual.pdf release/kaseta-build-manual.pdf

export CHANGES=$(awk "/## ${version}/{flag=1;next}/## */{flag=0}flag" CHANGELOG.md | awk 'NF')

envsubst < hack/release.tmpl.md > release/notes.md
