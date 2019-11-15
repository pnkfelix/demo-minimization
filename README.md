To observe the bug, run this:

```bash
% ( cd tock/boards/arty-e21/ && RUSTFLAGS="-C link-arg=-Tlayout.ld -C linker=rust-lld -C linker-flavor=ld.lld -C relocation-model=dynamic-no-pic -C link-arg=-zmax-page-size=512" cargo +nightly-2019-10-17 build  --target riscv32imac-unknown-none-elf   )
```
