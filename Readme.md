Build contract
-------------------------

``` bash
$ cargo build-bpf
$ cargo test-bpf
```

Deploy contract
-------------------------

``` bash
$ solana program deploy --upgrade-authority <synchronizer-keypair> --program-id <program_id-keypair> <path/to/synchronizer.so>
```
