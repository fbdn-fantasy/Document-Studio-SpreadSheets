#!/bin/sh
set -eu

if [ "$(uname -m)" != x86_64 ]; then
    echo "This release requires 64-bit Intel or AMD Linux (x86_64)." >&2
    exit 1
fi

directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)

# On machines with both package managers, use the distribution's native one.
if [ -r /etc/os-release ]; then
    . /etc/os-release
fi
manager=
for family in ${ID:-} ${ID_LIKE:-}; do
    case "$family" in
        debian|ubuntu) manager=apt-get; break ;;
        fedora|rhel|centos) manager=dnf; break ;;
        suse|opensuse*) manager=zypper; break ;;
    esac
done
if [ -z "$manager" ]; then
    for candidate in apt-get dnf zypper; do
        if command -v "$candidate" >/dev/null 2>&1; then
            manager=$candidate
            break
        fi
    done
fi

case "$manager" in
    apt-get) package="$directory/packages/document-studio-spreadsheets.deb" ;;
    dnf|zypper) package="$directory/packages/document-studio-spreadsheets.rpm" ;;
    *) echo "Installation requires APT, DNF, or Zypper." >&2; exit 1 ;;
esac
if ! command -v "$manager" >/dev/null 2>&1; then
    echo "Install $manager to use this installer." >&2
    exit 1
fi
if [ ! -f "$package" ]; then
    echo "Missing package: $package. Extract the complete archive and try again." >&2
    exit 1
fi

set -- "$manager" install "$package"
if [ "$(id -u)" != 0 ]; then
    if ! command -v sudo >/dev/null 2>&1; then
        echo "Run ./install.sh as an administrator." >&2
        exit 1
    fi
    set -- sudo "$@"
fi
exec "$@"
