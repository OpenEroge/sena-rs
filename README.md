# sena-rs

<img src="./assets/icon.png" alt="icon" align="left" width="100" style="margin-right: 10px;" />

Rust cross-platform implementation of the SoftPAL engine.
Specifically, targeted for the game "Koikake".

<br clear="left"/>

## Example screenshots

* sena-rs on macOS
![On Mac](./assets/screenshot-mac.png)

* sena-rs on iOS
![On iOS](./assets/screenshot-ios.png)

| Platform | Targets |
|---|---|
| Linux | x86_64, aarch64 |
| Windows | x86_64, ARM64 |
| macOS | DMG app bundle |
| iOS | arm64 device, arm64 simulator, x86_64 simulator |
| Android | arm64-v8a, x86_64 |
| WebAssembly | wasm32-unknown-unknown |
| Nintendo Switch | Homebrew NRO, experimental; package build verified |
| PlayStation Vita | Homebrew VPK, experimental; package build verified |

See [platform build instructions](platform/README.md) for SDK requirements,
controls, artifacts, and validation limits. Console hardware has not been tested.

## Run

```bash
cargo run -p pal-vm --release --bin sena -- /path/to/game --nls sjis
```

A game root that carries a `mai` fan-translation patch (`mai.dll` plus `mai/`)
can be run with that overlay applied; see
[reverse/mai-translation-patch.md](reverse/mai-translation-patch.md).

```bash
cargo run -p pal-vm --release --bin sena -- /path/to/game --patch mai
```

## License
This project is licensed under the MPL-2.0 License. See [LICENSE](./LICENSE-MPL-2.0) for details.
