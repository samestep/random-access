# Random access

Code supplement to my [blog post about random-access memory](https://samestep.com/blog/random-access/). By default, the `generate` and `measure` subcommands do powers of two through 24:

```sh
cargo run --release generate
cargo run --release measure | tee -a measurements.jsonl
```

There are various flags you can pass to configure the behavior; use `--help` to see what they are:

```sh
cargo run --release generate --help
cargo run --release measure --help
```
