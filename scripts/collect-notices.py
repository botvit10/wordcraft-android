"""Collect published crate notices for the normal ARM64 dependency tree."""
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

target = "aarch64-linux-android"
out = Path(sys.argv[1])
tree = subprocess.check_output(
    ["cargo", "tree", "--locked", "-p", "wordcraft-android", "--target", target,
     "--edges", "normal", "--prefix", "none", "--format", "{p}"], text=True
)
selected = {tuple(line.split()[:2]) for line in tree.splitlines()}
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform", target]
))
inventory = []
for package in metadata["packages"]:
    name, version = package["name"], package["version"]
    if (name, "v" + version) not in selected:
        continue
    inventory.append(f"{name}\t{version}\t{package['license'] or package['license_file'] or 'see source'}")
    if not package["source"]:
        continue  # Workspace and vendored winit notices are copied by ci-arm64.sh.
    root = Path(package["manifest_path"]).parent
    for path in root.rglob("*"):
        relative = path.relative_to(root)
        is_notice = re.search(r"licen[cs]e|copyright|copying|^notice|^(ofl|ufl)\b",
                              path.name, re.IGNORECASE)
        is_font_notice = name == "epaint_default_fonts" and path.suffix == ".txt"
        if is_notice or is_font_notice or str(relative) == package["license_file"]:
            if path.is_file() and not path.is_symlink() and path.suffix not in {".rs", ".py", ".sh"}:
                dest = out / f"{name}-{version}" / relative
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(path, dest)
(out / "DEPENDENCIES.tsv").write_text("crate\tversion\tlicense\n" + "\n".join(sorted(inventory)) + "\n")
