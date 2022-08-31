# Development

This project utilizes [cargo make](https://github.com/sagiegurari/cargo-make).
Start by installing it:

```sh
cargo install --force cargo-make
```

Run formatting, checks, build and tests:

```sh
cargo make dev
```

Run benchmarks:

```sh
cargo bench
```

Profiling:

``` sh
rm -f target/release/deps/bench-*
rm -f callgrind.out.*
TEST=processor
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
