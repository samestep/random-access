# Random access

Code supplement to my [blog post about random-access memory](https://samestep.com/blog/random-access/). By default, all the subcommands do powers of two through 24, which is significantly smaller than the data shown in the post.

```sh
cargo run --release generate
cargo run --release measure | tee -a ram.jsonl
cargo run --release measure --mmap | tee -a mmap.jsonl
cargo run --release sum | tee -a buffer.jsonl
```

There are various flags you can pass to configure the behavior; use `--help` to see what they are.

```sh
cargo run --release generate --help
cargo run --release measure --help
cargo run --release sum --help
```
