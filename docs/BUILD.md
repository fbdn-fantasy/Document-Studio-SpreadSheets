# Build Document Studio Spreadsheets

Run commands from the directory containing `package.json`. Use Node.js 22.12 or newer with npm, and a current stable Rust toolchain. The npm and Cargo lockfiles record the dependency versions used for this release.

## Prepare and test

Install the platform tools described below first.

```sh
npm ci
npm run check
npm run build
cargo test --locked --release --manifest-path src-tauri/Cargo.toml
```

Run `npm run tauri dev` to start the desktop app from source.

## Windows setup

On x64 Windows, install Microsoft C++ Build Tools with the Windows SDK, Rust for the MSVC toolchain and WebView2. Build the installer with

```powershell
npm run tauri build -- --bundles nsis
```

The setup file is written under `src-tauri/target/release/bundle/nsis/`. The first build needs internet access for any uncached dependencies, NSIS tools and the offline WebView2 installer.

To cross-build on Linux, install the `x86_64-pc-windows-msvc` Rust target, cargo-xwin, clang, lld, LLVM and NSIS, then run

```sh
npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis
```

Cross-built setup files are under `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/`. Setting `CARGO_TARGET_DIR` relocates the output for either build.

## Linux packages

Build on x86_64 Linux with a C/C++ compiler, pkg-config and the development libraries for GTK 3, WebKitGTK 4.1, OpenSSL and librsvg. For example, on Ubuntu the development packages include `build-essential`, `pkg-config`, `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libssl-dev` and `librsvg2-dev`. Install Python 3 to create the tar archive.

```sh
npm run tauri build -- --bundles deb,rpm
```

The packages are in `src-tauri/target/release/bundle/deb/` and `src-tauri/target/release/bundle/rpm/`. Use the matching DEB and RPM from the same build to make the tar installer.

```sh
python3 scripts/package_linux_tar.py src-tauri/target/release/bundle/deb/*.deb src-tauri/target/release/bundle/rpm/*.rpm release
```

The archive holds both packages, the license and an install script. APT, DNF or Zypper installs the matching package and its runtime dependencies. If you build on a different Linux baseline, check the linked libraries and minimum versions against the dependencies in `src-tauri/tauri.conf.json`.

## Source ZIP

```sh
python3 scripts/package_source.py ../document-studio-spreadsheets-source.zip "Document Studio Spreadsheets V10"
```

The script creates a compressed source ZIP and checks the stored files against their SHA-256 hashes.
