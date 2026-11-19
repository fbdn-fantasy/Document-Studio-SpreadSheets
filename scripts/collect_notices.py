"""Collect dependency license notices from locked Cargo/npm dependency sources."""
from pathlib import Path
import json
import subprocess

root = Path(__file__).resolve().parents[1]
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--manifest-path", str(root / "src-tauri/Cargo.toml"),
    "--format-version", "1", "--locked"
], text=True))
parts = ["# Third-party notices\n\nGenerated from the locked dependency graph. "
         "This includes development/target-specific dependencies conservatively. "
         "The application uses the supplied user-owned icon.\n\n"
         "The Support Me button uses the official Ko-fi cup logo from the "
         "[Ko-fi creator kit](https://more.ko-fi.com/brand-assets).\n"]
licenses = []
for p in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
    if p["name"] == "document-studio-spreadsheets":
        continue
    directory = Path(p["manifest_path"]).parent
    parts.append(f"- {p['name']} {p['version']}: {p.get('license') or 'see included license text'}; {p.get('repository') or p.get('homepage') or ''}")
    for path in directory.iterdir():
        if path.is_file() and (path.name.upper().startswith(("LICENSE", "COPYING", "NOTICE")) or path.name == "THIRD-PARTY-NOTICES.md"):
            licenses.append(f"\n{'=' * 72}\n{p['name']} {p['version']} — {path.name}\n{'=' * 72}\n" + path.read_text(errors="replace"))
lock = json.loads((root / "package-lock.json").read_text())
for relative, entry in sorted(lock["packages"].items()):
    if not relative:
        continue
    directory = root / relative
    parts.append(f"- npm {relative.removeprefix('node_modules/')} {entry.get('version', '')}: {entry.get('license', 'see package source')}")
    if directory.is_dir():
        for path in directory.iterdir():
            if path.is_file() and path.name.upper().startswith(("LICENSE", "COPYING", "NOTICE")):
                licenses.append(f"\n{'=' * 72}\n{relative} — {path.name}\n{'=' * 72}\n" + path.read_text(errors="replace"))
(root / "docs/THIRD-PARTY-NOTICES.md").write_text("\n".join(parts), encoding="utf-8")
(root / "docs/THIRD-PARTY-LICENSE-TEXTS.txt").write_text("\n".join(licenses), encoding="utf-8")
print(f"Collected {len(licenses)} license/notice texts")
