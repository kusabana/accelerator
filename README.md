<div align="center">
  <h3><a href="https://github.com/kusabana">
    ~kusabana/</a>accelerator
  </h3>
multithreads source engine http downloads
</div>

## Installation
1. download the latest artifact or compile it yourself
2. move the binary module (.dll file) to `garrysmod/lua/bin`  
3. add `require'accelerator'` to `garrysmod/lua/menu/menu.lua` in order to load the module on launch
## Building
```sh
git clone https://github.com/kusabana/accelerator
cd accelerator
cargo +nightly build --release --target=<desired_target>
```

target triples:
```sh
x86_64-unknown-linux-gnu # linux 64-bit
x86_64-pc-windows-msvc # windows 64-bit
i686-unknown-linux-gnu # linux 32-bit
i686-pc-windows-msvc # windows 32-bit
```
## Todo
```
- check hash instead of giving up when file exists
```