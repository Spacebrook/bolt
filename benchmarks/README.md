# Benchmarks

Run the Rust benchmark suite:

```
cargo run -p benchmarks
```

Include the C reference benchmark:

```
BOLT_BENCH_FILTER=c-quadtree cargo run -p benchmarks
```

The C benchmark is compiled in headless mode with the upstream `test.c` settings.

Environment variables:
- `BOLT_C_QUADTREE_DIR`: use an existing checkout instead of cloning.
- `BOLT_C_QUADTREE_REPO`: override the clone URL (default: `https://github.com/supahero1/c-quadtree`).
- `BOLT_C_QUADTREE_UPDATE`: set to `1` to `git pull` the repo before building.
