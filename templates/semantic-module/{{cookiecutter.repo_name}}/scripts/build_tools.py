#!/usr/bin/env python3
"""Build and versioning scripts for {{ cookiecutter.package_name }}."""

import hashlib
import os
import subprocess
import sys
from datetime import datetime
from pathlib import Path


def run(cmd: str) -> str:
    """Run command and return stripped output."""
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    return result.stdout.strip()


def git_tag() -> str:
    """Get latest git tag or default."""
    tag = run("git describe --tags --abbrev=0 2>/dev/null")
    return tag if tag else "0.0.0"


def git_commit() -> str:
    """Get short commit hash."""
    commit = run("git rev-parse --short HEAD 2>/dev/null")
    return commit if commit else "local"


def git_branch() -> str:
    """Get sanitized branch name."""
    branch = run("git rev-parse --abbrev-ref HEAD 2>/dev/null")
    if not branch:
        return "local"
    return "".join(c if c.isalnum() else "-" for c in branch)


def git_status() -> str:
    """Check if repo is dirty or clean."""
    diff = run("git diff --no-ext-diff 2>/dev/null")
    untracked = run("git ls-files --others --exclude-standard 2>/dev/null")
    return "dirty" if diff or untracked else "clean"


def dirty_hash() -> str:
    """Hash of uncommitted changes."""
    diff = run("git diff --no-ext-diff 2>/dev/null")
    untracked_files = run("git ls-files --others --exclude-standard 2>/dev/null")
    content = diff
    for f in untracked_files.split("\n"):
        if f and Path(f).exists():
            content += Path(f).read_text(errors="ignore")
    return hashlib.sha256(content.encode()).hexdigest()[:8]


def full_version() -> str:
    """Compute full PEP440-compliant version."""
    tag = git_tag().lstrip("v")
    status = git_status()
    commit = git_commit()

    if status == "dirty":
        timestamp = datetime.now().strftime("%Y%m%d.%H%M%S")
        return f"{tag}+{timestamp}.{commit}.{dirty_hash()}"

    # Check if we're exactly on a tag
    exact = run("git describe --tags --exact-match 2>/dev/null")
    if exact:
        return tag

    return f"{tag}+{commit}"


def cmd_version():
    """Print computed version."""
    print(full_version())


def cmd_info():
    """Print git and version info."""
    status = git_status()
    print(f"Tag:        {git_tag()}")
    print(f"Commit:     {git_commit()}")
    print(f"Branch:     {git_branch()}")
    print(f"Status:     {status}")
    if status == "dirty":
        print(f"Dirty hash: {dirty_hash()}")
    print(f"Local Ver:  {full_version()}")
    poetry_ver = run("poetry version -s 2>/dev/null")
    print(f"Poetry Ver: {poetry_ver or 'N/A'}")


def cmd_build():
    """Build distribution with computed version."""
    version = full_version()
    print(f"📦 Building distribution with version {version}...")

    # Clean first
    for pattern in ["dist/", "*.egg-info/"]:
        run(f"rm -rf {pattern}")

    # Build with version bypass
    env = os.environ.copy()
    env["POETRY_DYNAMIC_VERSIONING_BYPASS"] = version
    result = subprocess.run(["poetry", "build"], env=env)
    sys.exit(result.returncode)


def cmd_clean():
    """Remove build artifacts."""
    patterns = [
        "dist/",
        "*.egg-info/",
        "__pycache__/",
        "{{ cookiecutter.package_name }}/__pycache__",
        "tests/__pycache__",
        "scripts/__pycache__",
        ".pytest_cache/",
        ".coverage",
        "report.json",
    ]
    for p in patterns:
        run(f"rm -rf {p}")
    print("✨ Cleaned build artifacts")


def main():
    """CLI entry point."""
    if len(sys.argv) < 2:
        print("Usage: build-tools <version|info|build|clean>")
        sys.exit(1)

    cmd = sys.argv[1]
    commands = {
        "version": cmd_version,
        "info": cmd_info,
        "build": cmd_build,
        "clean": cmd_clean,
    }

    if cmd not in commands:
        print(f"Unknown command: {cmd}")
        print(f"Available: {', '.join(commands.keys())}")
        sys.exit(1)

    commands[cmd]()


if __name__ == "__main__":
    main()
