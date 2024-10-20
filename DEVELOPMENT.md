# Development

## Environment setup

Follow [Rust's installation guide](https://www.rust-lang.org/tools/install).

Install tooling of the embedded Rust target for Cortex-M7F. It is necessary to
use an older version of Rust, since newer version bloat the firmware:

```sh
rustup install 1.77.0
rustup +1.77.0 target add thumbv7em-none-eabihf
```

This project utilizes [cargo make](https://github.com/sagiegurari/cargo-make):

```sh
cargo +1.77.0 install cargo-make
```

Furthermore, the embedded part of the project uses [flip
link](https://github.com/knurling-rs/flip-link):

```sh
cargo +1.77.0 install flip-link
```

## Formatting, linting, unit tests

Run formatting, linter and unit tests:

```sh
cargo +1.77.0 make
```

## DSP benchmark

Run the benchmark:

``` sh
cd dsp
cargo +1.77.0 bench --bench bench
```

Use a profiler to analyze the results:

``` sh
cd dsp
rm -f target/release/deps/bench-*
rm -f callgrind.out.*
RUSTFLAGS="-g" cargo +1.77.0 bench --bench bench --no-run
BENCH=$(find target/release/deps -type f -executable -name 'bench-*')
valgrind \
    --tool=callgrind \
    --dump-instr=yes \
    --collect-jumps=yes \
    --simulate-cache=yes \
    ${BENCH} --bench --profile-time 10
kcachegrind callgrind.out.*
```

## Flash via ST-Link

This requires external probe, such as the ST LINK-V3 MINI. The benefit of this
approach is that it allows to stay connected to the module, read logs, run a
debugger, or execute tests on the module. Note that the module needs to be
powered while the probe is connected.

This project uses [probe-rs](https://github.com/probe-rs/probe-rs) to deal with
flashing. Start by installing its dependencies. For Fedora, it can be done by
running the following:

```sh
sudo dnf install libusbx-devel libftdi-devel libudev-devel
```

You may then install needed udev rules. See the [probe-rs getting
started](https://probe.rs/docs/getting-started/probe-setup/) to learn how.

Then install Rust dependencies of probe-rs:

```sh
cargo +1.77.0 install probe-run
cargo +1.77.0 install flip-link
```

To flash the project, call this make target:

```sh
cargo +1.77.0 make flash
```

Logging level can be set using an environment variable:

```sh
DEFMT_LOG=info cargo +1.77.0 make flash
```

## Flash via DFU

Unlike ST-Link, DFU flashing does not require any external probe. Just connect
the module to your computer via a USB cable.

First, install [dfu-util](http://dfu-util.sourceforge.net/) and
[cargo-binutils](https://github.com/rust-embedded/cargo-binutils).
On Fedora, this can be done by calling:

```sh
sudo dnf install dfu-util
cargo +1.77.0 install cargo-binutils
rustup +1.77.0 component add llvm-tools-preview
```

Click the RESET button while holding the BOOT button of the Daisy Patch SM to
enter the bootloader. Then call this make target:

```sh
cargo +1.77.0 make flash-dfu
```

## Embedded tests

Firmware integration tests are executed directly on the module.

Before running an embedded test, first make sure to go through the guidance
given in [Flash via ST-Link](#flash-via-st-link).

To run one of the integration tests kept under `firmware/tests`:

```sh
cargo +1.77.0 make test-embedded button
```

## Embedded benchmark

For the most accurate results, benchmarks of control and dsp modules are executed
directly against the hardware.

Before running a benchmark, first make sure to go through the guidance given in
[Flash via ST-Link](#flash-via-st-link).

To run one of the benchmarks kept under `benches/src/bin`:

```sh
cargo +1.77.0 make bench oversampling
```

## Firmware size

Daisy Patch SM can fit up to 128 kB of firmware. It is important to make sure that
the firmware size stays slim and no bloat gets in.

Install needed tooling:

```sh
cargo +1.77.0 install cargo-bloat
cargo +1.77.0 install cargo-binutils
rustup +1.77.0 component add llvm-tools-preview
```

Run the following command often to make sure no unnecessary heavy dependencies
are brought in:

```sh
cargo +1.77.0 make bloat
```

## Manual

The user manual is defined in latex under `manual/user`. To build it, first
install needed pre-requisites. On Fedora it can be done by running the
following:

```sh
sudo dnf install inkscape texlive-latex texlive-ec texlive-microtype texlive-pagecolor texlive-parskip texlive-titling texlive-hardwrap texlive-mdwtools
```

To build the manual:

```sh
cargo +1.77.0 make manual
```

The built PDF is then available in `manual/user/manual.pdf`.

## Gerbers, BOM and CPL

I extensivelly use https://github.com/Bouni/kicad-jlcpcb-tools to deal with the
matters listed in the title, and to prepare project for manufacture.
