"""Produce only the reproducible source archive; no binaries or build caches."""
from pathlib import Path
import zipfile
import sys
import os
import hashlib
import json

root = Path(__file__).resolve().parents[1]
destination = Path(sys.argv[1]).resolve()
archive_root = sys.argv[2] if len(sys.argv) > 2 else root.name
excluded = {"node_modules", "target", "dist", "release", ".git", "__pycache__", "installed-v3", "startup-test-profile"}
destination.parent.mkdir(parents=True, exist_ok=True)
manifest = {}
with zipfile.ZipFile(destination, "x", zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
    for directory, directories, files in os.walk(root):
        directories[:] = sorted(d for d in directories if d not in excluded)
        if Path(directory) == root / "src-tauri" / "gen" and "schemas" in directories:
            directories.remove("schemas")
        for filename in sorted(files):
            path = Path(directory) / filename
            relative = path.relative_to(root)
            if path.suffix not in {".log", ".zip"} and filename != "SOURCE-MANIFEST.json":
                name = (Path(archive_root) / relative).as_posix()
                manifest[name] = hashlib.sha256(path.read_bytes()).hexdigest()
                archive.write(path, name)
    archive.writestr(archive_root + "/SOURCE-MANIFEST.json", json.dumps(manifest, indent=2))
with zipfile.ZipFile(destination, "r") as archive:
    assert archive.testzip() is None, "ZIP integrity check failed"
    assert set(archive.namelist()) == set(manifest) | {archive_root + "/SOURCE-MANIFEST.json"}
    for name, digest in manifest.items():
        assert hashlib.sha256(archive.read(name)).hexdigest() == digest, name
print(f"Verified {len(manifest)} source files with CRC and SHA-256")
print(destination)

