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

Build on x86_64 Linux with a C/C++ compiler, pkg-config and the development libraries for GTK 3, WebKitGTK 4.1, OpenSSL and librsvg. On Ubuntu the development packages include `build-essential`, `pkg-config`, `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libssl-dev` and `librsvg2-dev`. Packaging requires Python 3, dpkg-deb, rpmbuild and desktop-file-validate.

```sh
npm run tauri build -- --bundles deb -- --locked
python3 scripts/package_linux.py src-tauri/target/release/bundle/deb/*.deb release/Linux
```

The packaging script creates the release DEB, RPM and TAR under `release/Linux`. The native packages use epoch 1 so that this version series upgrades the old 1.8.0 release. The menu entry uses the app's GTK identifier and a stable launcher across upgrades. The TAR includes both native packages, LICENSE, NOTICE and an install script. APT, DNF or Zypper installs the matching package and its runtime dependencies. If you change the Linux build baseline, check the linked libraries and minimum versions against `src-tauri/tauri.conf.json`.

## Source ZIP

```sh
python3 scripts/package_source.py ../Document-Studio-Spreadsheets-V1.0.0-Windows-Source.zip Document-Studio-Spreadsheets-V1.0.0
```

The script creates a compressed source ZIP and checks the stored files against their SHA-256 hashes.

## Publishing an update

Use a published GitHub release in `fbdn-fantasy/Document-Studio-SpreadSheets`. For example, the next patch release can use the tag `V1.0.1`. Set the same numeric version in the npm and Cargo manifests and lockfiles, the Tauri version, and the versioned Tauri executable name.

Upload the installers as individual release assets. For V1.0.1 their names are

- `Document-Studio-Spreadsheets-V1.0.1-Setup.exe`
- `Document-Studio-Spreadsheets-V1.0.1.deb`
- `document-studio-spreadsheets-1.0.1-1.x86_64.rpm`

Attach each package’s corresponding source archive and the installable Linux TAR as well. The TAR installs the native DEB or RPM, so those installations receive the matching package on update. Publish after all assets have finished uploading.

**Check For Updates** contacts GitHub only on a click. It compares stable three-part versions, chooses the matching installer, verifies its size and GitHub-provided SHA-256 digest, and starts the system installer. The previous `V10` tag belongs to the older numbering series. Install V1.0.0 manually to switch from that series.
