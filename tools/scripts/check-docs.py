#!/usr/bin/env python3
"""Check markdown relative links and table column consistency."""
import glob
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
os.chdir(ROOT)

FILES = ["README.md", "README.zh-CN.md"] + sorted(glob.glob("docs/*.md"))


def slug(text: str) -> str:
    text = text.strip().lower()
    text = re.sub(r"<[^>]+>", "", text)
    text = re.sub(r"[^\w\u4e00-\u9fff\s-]", "", text)
    return text.replace(" ", "-")


anchors = {}
for path in FILES:
    with open(path, encoding="utf-8") as handle:
        anchors[path] = {slug(line) for line in re.findall(r"^#{1,6}\s+(.*)$", handle.read(), re.M)}

broken = []
for path in FILES:
    with open(path, encoding="utf-8") as handle:
        content = handle.read()
    base = os.path.dirname(path)
    for match in re.finditer(r"\[([^\]]*)\]\(([^)]+)\)", content):
        url = match.group(2)
        if url.startswith(("http", "mailto:", "#")):
            continue
        target, _, fragment = url.partition("#")
        if not target:
            continue
        resolved = os.path.normpath(os.path.join(base, target))
        if not os.path.exists(resolved):
            broken.append(f"{path}: missing file {url}")
        elif fragment and resolved in anchors and fragment not in anchors[resolved]:
            broken.append(f"{path}: missing anchor {url}")

bad_tables = []
for path in FILES:
    lines = open(path, encoding="utf-8").read().splitlines()
    block = []
    for index, line in enumerate(lines + [""]):
        if line.strip().startswith("|"):
            block.append(line)
        else:
            if len(block) > 2 and len({line.count("|") for line in block}) > 1:
                bad_tables.append(f"{path}:{index - len(block) + 1}")
            block = []

for item in broken:
    print("BROKEN", item)
for item in bad_tables:
    print("TABLE", item)
print(f"checked {len(FILES)} files: broken_links={len(broken)} bad_tables={len(bad_tables)}")
sys.exit(1 if broken or bad_tables else 0)
