#!/usr/bin/env python3
"""校验稳定版本 tag，并从 CHANGELOG.md 提取本次发布说明。"""
import os
import re
import sys
import tomllib
from pathlib import Path


def prepare(tag: str, notes_path: str) -> None:
    match = re.fullmatch(r"v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", tag)
    if not match:
        raise ValueError("tag 必须使用 vX.Y.Z")
    major, minor, patch = map(int, match.groups())
    code = major * 1_000_000 + minor * 1_000 + patch
    if minor >= 1000 or patch >= 1000 or not 1 <= code <= 2_100_000_000:
        raise ValueError("版本号超出 Android versionCode 范围")
    version = tag[1:]
    manifest = tomllib.loads(Path("Cargo.toml").read_text())
    if manifest["workspace"]["package"]["version"] != version:
        raise ValueError("tag 与 Cargo.toml 版本不一致")
    changelog = Path("CHANGELOG.md").read_text()
    section = re.search(
        rf"^## \[{re.escape(version)}\] - \d{{4}}-\d{{2}}-\d{{2}}\n(.*?)(?=^## |\Z)",
        changelog, re.M | re.S,
    )
    if not section or not re.search(r"^- \S", section[1], re.M):
        raise ValueError("缺少对应版本的有效 changelog")
    Path(notes_path).write_text(section[1].strip() + "\n")
    if env := os.environ.get("GITHUB_ENV"):
        with open(env, "a") as file:
            file.write(f"APP_VERSION={version}\nAPP_VERSION_CODE={code}\n")
    print(f"Validated {tag} (Android versionCode {code})")


if __name__ == "__main__":
    prepare(*sys.argv[1:])
