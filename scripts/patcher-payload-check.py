#!/usr/bin/env python3
"""Verify that the RO-Next patcher payload ships everything the client needs.

The patcher publishes an immutable release with a manifest of `(path, sha256)`
entries (see `scripts/publish-content-release.sh` on the patcher host). Nothing
in the client build fails when a data file is simply not in that manifest: the
loader logs a warning and the feature silently disappears. That happened once
(the two damage-number files, the monster-motion table and the status-launch
table were all missing from release 0.2.29 because they were added after the
payload was last synced), so this script turns "did the patcher pick up the
change?" into a check with an exit code.

It compares the *sources of truth in this repo* against the published manifest:

    ro-next/archive/**                      -> archive/**
    ro-next/client/game_archives.ron         -> client/game_archives.ron
    ro-next/client/interface_settings.ron    -> defaults/interface_settings.ron
    <windows exe>                            -> RO-Next.exe

Usage:
    # after a release, against the live API
    scripts/patcher-payload-check.py --exe target/x86_64-pc-windows-msvc/release/ro-next.exe

    # against a manifest file (offline / CI)
    scripts/patcher-payload-check.py --manifest /tmp/ro_next-manifest.json \
        --exe target/x86_64-pc-windows-msvc/release/ro-next.exe --expect-version 0.2.30

Exit codes: 0 = payload is complete, 1 = missing/stale files or wrong version,
2 = the manifest could not be read.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import ssl
import sys
import urllib.request
from pathlib import Path

DEFAULT_API = "https://rathena.uspectr.com/patcher-server/api"

#: Payload files that are *deployment-specific*: the released
#: `client/game_archives.ron` points at `../game/*.grf` (the patcher install
#: layout, GRFs live next to the client directory) and the seed settings file is
#: the release default, not the developer's local one. Only their presence is
#: required; their hashes are expected to differ from the repo copies.
PRESENCE_ONLY = ("client/game_archives.ron", "defaults/interface_settings.ron")

#: payload path of the Windows client (the release is built for Windows)
EXE_PAYLOAD_PATH = "RO-Next.exe"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_manifest(arguments: argparse.Namespace) -> dict:
    if arguments.manifest:
        try:
            return json.loads(Path(arguments.manifest).read_text(encoding="utf-8"))
        except (OSError, ValueError) as error:
            print(f"error: cannot read manifest {arguments.manifest}: {error}", file=sys.stderr)
            raise SystemExit(2)

    url = f"{arguments.api_base}/manifest?client={arguments.client}"
    # The patcher API uses the host's certificate; the release scripts curl it
    # with `-k` for the same reason.
    context = ssl._create_unverified_context()
    try:
        with urllib.request.urlopen(url, timeout=60, context=context) as response:
            return json.loads(response.read().decode("utf-8"))
    except Exception as error:  # noqa: BLE001 - report the real failure to the operator
        print(f"error: cannot fetch {url}: {error}", file=sys.stderr)
        raise SystemExit(2)


def expected_files(repo_root: Path) -> dict[str, Path]:
    files: dict[str, Path] = {}

    archive = repo_root / "ro-next" / "archive"
    if not archive.is_dir():
        print(f"error: archive directory not found: {archive}", file=sys.stderr)
        raise SystemExit(2)
    for path in sorted(archive.rglob("*")):
        if path.is_file():
            files["archive/" + path.relative_to(archive).as_posix()] = path

    return files


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--client", default="ro_next")
    parser.add_argument("--api-base", default=DEFAULT_API)
    parser.add_argument("--manifest", help="read a manifest JSON file instead of the live API")
    parser.add_argument("--repo-root", default=str(Path(__file__).resolve().parents[1]))
    parser.add_argument("--exe", help="Windows client binary to compare against RO-Next.exe")
    parser.add_argument("--expect-version", help="fail unless the manifest version equals this")
    parser.add_argument("--quiet-extra", action="store_true", help="do not list payload-only files")
    arguments = parser.parse_args()

    manifest = load_manifest(arguments)
    published = {entry["path"]: entry for entry in manifest.get("files", [])}

    problems: list[str] = []

    version = manifest.get("version")
    if arguments.expect_version and str(version) != str(arguments.expect_version):
        problems.append(f"version: manifest says {version!r}, expected {arguments.expect_version!r}")

    repo_root = Path(arguments.repo_root)
    local = expected_files(repo_root)
    for payload_path, local_path in sorted(local.items()):
        entry = published.get(payload_path)
        if entry is None:
            problems.append(f"missing from payload: {payload_path} ({local_path.relative_to(repo_root)})")
            continue
        digest = sha256(local_path)
        if entry.get("hash") != f"sha256:{digest}":
            problems.append(
                f"stale in payload: {payload_path} "
                f"(local {digest[:12]}, payload {str(entry.get('hash'))[:19]})"
            )

    for payload_path in PRESENCE_ONLY:
        if payload_path not in published:
            problems.append(f"missing from payload: {payload_path} (deployment-specific file)")

    if arguments.exe:
        exe = Path(arguments.exe)
        if not exe.is_file():
            print(f"error: executable not found: {exe}", file=sys.stderr)
            return 2
        entry = published.get(EXE_PAYLOAD_PATH)
        digest = sha256(exe)
        if entry is None:
            problems.append(f"missing from payload: {EXE_PAYLOAD_PATH}")
        elif entry.get("hash") != f"sha256:{digest}":
            problems.append(
                f"stale in payload: {EXE_PAYLOAD_PATH} "
                f"(local {digest[:12]}, payload {str(entry.get('hash'))[:19]})"
            )

    extra = sorted(set(published) - set(local) - set(PRESENCE_ONLY) - ({EXE_PAYLOAD_PATH} if arguments.exe else set()))
    if extra and not arguments.quiet_extra:
        print(f"payload-only files (not shipped from this repo, informational): {len(extra)}")
        for path in extra:
            print(f"  {path}")

    print(f"manifest version {version}, release {manifest.get('release_id')}, {len(published)} files")

    if problems:
        print(f"FAIL: {len(problems)} problem(s)")
        for problem in problems:
            print(f"  {problem}")
        return 1

    print("OK: the payload ships every file this repo expects")
    return 0


if __name__ == "__main__":
    sys.exit(main())
