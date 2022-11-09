# Development

This project utilizes [cargo make](https://github.com/sagiegurari/cargo-make).
Start by installing it:

```sh
cargo install --force cargo-make
```

Furthermore, the embedded part of the project uses [flip
link](https://github.com/knurling-rs/flip-link). Install it:

```sh
cargo install flip-link
```

Run formatting, checks, build and tests:

```sh
cargo make dev
```

Run software benchmarks:

```sh
cargo bench
```

Profiling:

``` sh
TEST=processor
rm -f target/release/deps/${TEST}-*
rm -f callgrind.out.*
RUSTFLAGS="-g" cargo bench --bench ${TEST} --no-run
BENCH=$(find target/release/deps -type f -executable -name "${TEST}-*")
valgrind \
    --tool=callgrind \
    --dump-instr=yes \
    --collect-jumps=yes \
    --simulate-cache=yes \
    ${BENCH} --bench --profile-time 10 ${TEST}
kcachegrind callgrind.out.*
```

Run benchmarks on hardware:

```sh
cd benches
cargo run --bin hello
```

## Gerbers, BOM and CPL

I extensivelly use https://github.com/Bouni/kicad-jlcpcb-tools to deal with the
matters listed in the title, and to prepare project for manufacture.
