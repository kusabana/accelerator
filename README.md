<div align="center">
  <h3><a href="https://github.com/ezekielathome">
    ~ezekielathome/</a>accelerator
  </h3>
multithreads source engine http downloads
</div>

## Installation
1. download the latest [artifact](https://nightly.link/ezekielathome/accelerator/workflows/ci/trunk?preview) or compile it yourself
2. move the binary module (.dll file) to `garrysmod/lua/bin`  
3. add `require'accelerator'` to `garrysmod/lua/menu/menu.lua` in order to load the module on launch
## Building
```sh
git clone https://github.com/ezekielathome/accelerator
cd accelerator
cargo +nightly build --release --target=<desired_target>
```

target triples:
```sh
i686-unknown-linux-gnu # linux 32-bit
x86_64-unknown-linux-gnu # linux 64-bit
i686-pc-windows-msvc # windows 32-bit
x86_64-pc-windows-msvc # windows 64-bit
```

## Updating patterns
included is a simple script that uses HLIL pattern scanning to get up to date function addresses which you can use to generate the patterns.
