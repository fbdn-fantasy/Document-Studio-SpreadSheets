"""Bundle the DEB and RPM from one release into an installable Linux tarball."""

import argparse
import gzip
from io import BytesIO
import json
from pathlib import Path
import tarfile
from tempfile import TemporaryDirectory


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("deb", type=Path, help="Path to the release's amd64 DEB")
    parser.add_argument("rpm", type=Path, help="Path to the matching x86_64 RPM")
    parser.add_argument("output_dir", type=Path)
    args = parser.parse_args()

    for path, magic in [(args.deb, b"!<arch>\n"), (args.rpm, b"\xed\xab\xee\xdb")]:
        with path.open("rb") as package:
            if package.read(len(magic)) != magic:
                parser.error(f"Unrecognized package format: {path}")

    root = Path(__file__).resolve().parents[1]
    installer = root / "scripts" / "linux-installer"
    version = json.loads((root / "package.json").read_text(encoding="utf-8"))["version"]
    name = f"document-studio-spreadsheets-{version}-linux-x86_64"
    args.output_dir.mkdir(parents=True, exist_ok=True)
    destination = args.output_dir / f"{name}.tar.gz"

    # Fixed timestamps, owners and modes also make Windows-built archives usable on Linux.
    files = [
        ("install.sh", installer / "install.sh", 0o755),
        ("INSTALL.txt", installer / "INSTALL.txt", 0o644),
        ("LICENSE", root / "LICENSE", 0o644),
        ("packages/document-studio-spreadsheets.deb", args.deb, 0o644),
        ("packages/document-studio-spreadsheets.rpm", args.rpm, 0o644),
    ]
    with TemporaryDirectory(dir=args.output_dir) as work:
        temporary = Path(work) / destination.name
        with temporary.open("wb") as raw, gzip.GzipFile(filename="", fileobj=raw, mode="wb", mtime=0) as gz:
            with tarfile.open(fileobj=gz, mode="w|", format=tarfile.USTAR_FORMAT) as archive:
                for entry, source, mode in files:
                    data = source.read_bytes()
                    if entry == "install.sh":
                        data = data.replace(b"\r\n", b"\n")
                    info = tarfile.TarInfo(f"{name}/{entry}")
                    info.mode = mode
                    info.size = len(data)
                    archive.addfile(info, BytesIO(data))
        temporary.replace(destination)
    print(destination)


if __name__ == "__main__":
    main()
