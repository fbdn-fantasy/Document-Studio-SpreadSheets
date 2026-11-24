"""Create the public DEB, RPM and installable TAR from Tauri's x86_64 DEB."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory


def run(*args):
    subprocess.run(args, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tauri_deb", type=Path)
    parser.add_argument("linux_output", type=Path, help="Release Linux directory")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    config = json.loads((root / "src-tauri/tauri.conf.json").read_text())
    package = json.loads((root / "package.json").read_text())["name"]
    version = config["version"]
    app_id = config["identifier"]
    binary = config["mainBinaryName"]
    filename = f"Document-Studio-Spreadsheets-V{version}"
    deb = args.linux_output / "DEB" / f"{filename}.deb"
    rpm = args.linux_output / "RPM" / f"{package}-{version}-1.x86_64.rpm"
    deb.parent.mkdir(parents=True, exist_ok=True)
    rpm.parent.mkdir(parents=True, exist_ok=True)

    with TemporaryDirectory() as temporary:
        work = Path(temporary)
        payload = work / "payload"
        run("dpkg-deb", "--raw-extract", args.tauri_deb, payload)
        control = payload / "DEBIAN/control"
        metadata = control.read_text()
        fields = dict(re.findall(r"^(Package|Architecture|Version): (.+)$", metadata, re.M))
        if fields != {"Package": package, "Architecture": "amd64", "Version": version}:
            parser.error("The input DEB must match this application's name, architecture and version")

        # The version series restarts at 1.0.0 after the previous 1.8.0 package.
        control.write_text(metadata.replace(f"Version: {version}\n", f"Version: 1:{version}\n"))
        bins = payload / "usr/bin"
        if not (bins / binary).is_file():
            parser.error("The expected versioned application executable is missing")
        (bins / package).symlink_to(binary)
        desktop_dir = payload / "usr/share/applications"
        generated, = desktop_dir.glob("*.desktop")
        desktop = desktop_dir / f"{app_id}.desktop"
        generated.unlink()
        desktop.write_text(
            "[Desktop Entry]\nType=Application\n"
            f"Name={config['productName']}\nComment={config['bundle']['shortDescription']}\n"
            f"Exec={package} %F\nIcon={app_id}\nStartupWMClass={app_id}\n"
            "Terminal=false\nCategories=Office;Spreadsheet;\nStartupNotify=true\n"
            "MimeType=application/vnd.openxmlformats-officedocument.spreadsheetml.sheet;"
            "application/vnd.oasis.opendocument.spreadsheet;\n"
        )
        for icon in (payload / "usr/share/icons/hicolor").glob("*/apps/*.png"):
            icon.rename(icon.with_name(app_id + ".png"))
        files = sorted(p for p in (payload / "usr").rglob("*") if p.is_file() and not p.is_symlink())
        (payload / "DEBIAN/md5sums").write_text("".join(
            f"{hashlib.md5(p.read_bytes()).hexdigest()}  {p.relative_to(payload)}\n" for p in files
        ))
        run("desktop-file-validate", desktop)
        run("dpkg-deb", "-Zxz", "--root-owner-group", "--build", payload, work / deb.name)

        for target, source in config["bundle"]["linux"]["rpm"]["files"].items():
            path = payload / target.lstrip("/")
            path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(root / "src-tauri" / source, path)
        dependencies = "\n".join(f"Requires: {item}" for item in config["bundle"]["linux"]["rpm"]["depends"])
        paths = "\n".join(f'"/{p.relative_to(payload)}"' for p in sorted((payload / "usr").rglob("*")) if p.is_file())
        spec = work / "release.spec"
        spec.write_text(f"""%global debug_package %{{nil}}
%global _build_id_links none
Name: {package}
Epoch: 1
Version: {version}
Release: 1
Summary: {config['bundle']['shortDescription']}
License: {config['bundle']['license']}
BuildArch: x86_64
Packager: Forbidden_Fantasy
{dependencies}

%description
{config['productName']}

%install
mkdir -p "%{{buildroot}}"
cp -a "{payload}/usr" "%{{buildroot}}/usr"

%files
%defattr(-,root,root,-)
{paths}
""")
        run("rpmbuild", "-bb", "--define", f"_topdir {work}/rpmbuild",
            "--define", "_buildhost localhost", spec)
        shutil.copyfile(work / deb.name, deb)
        shutil.copyfile(work / "rpmbuild/RPMS/x86_64" / rpm.name, rpm)
    run(sys.executable, root / "scripts/package_linux_tar.py", deb, rpm, args.linux_output / "TAR")


if __name__ == "__main__":
    main()
