#!/usr/bin/env python3
"""
split_amproj.py — Split an amproj into individual element amprojs.

Each top-level child element of the <scene> becomes its own amproj.
Groups (embedScene) retain their entire subtree.

Usage:
    python3 dev/split_amproj.py <input.amproj> [output_dir]

If output_dir is not specified, creates a directory next to the input file
named <input_stem>_split/.
"""

import sys
import os
import uuid
import zipfile
import xml.etree.ElementTree as ET
from copy import deepcopy
from pathlib import Path


def split_amproj(input_path: str, output_dir: str | None = None):
    input_path = Path(input_path)
    if not input_path.exists():
        print(f"Error: {input_path} not found")
        sys.exit(1)

    if output_dir is None:
        output_dir = input_path.parent / f"{input_path.stem}_split"
    else:
        output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    # Read XML from amproj (zip)
    with zipfile.ZipFile(input_path, "r") as zf:
        xml_names = [n for n in zf.namelist() if n.endswith(".xml")]
        if not xml_names:
            print("Error: No XML file found in amproj")
            sys.exit(1)
        xml_content = zf.read(xml_names[0]).decode("utf-8")

    # Parse
    # Keep the XML declaration and comment
    header_lines = []
    xml_body_start = 0
    for i, line in enumerate(xml_content.split("\n")):
        if line.strip().startswith("<scene ") or line.strip().startswith("<scene>"):
            xml_body_start = i
            break
        header_lines.append(line)
    header = "\n".join(header_lines) + "\n"

    root = ET.fromstring(xml_content)
    if root.tag != "scene":
        print(f"Error: Root element is <{root.tag}>, expected <scene>")
        sys.exit(1)

    children = list(root)
    if not children:
        print("No child elements found in scene")
        sys.exit(0)

    print(f"Found {len(children)} top-level elements in {input_path.name}")

    for i, child in enumerate(children):
        label = child.get("label", child.get("id", f"element_{i}"))
        tag = child.tag
        # Sanitize filename
        safe_label = "".join(c if c.isalnum() or c in "-_ " else "_" for c in label).strip()
        safe_label = safe_label.replace(" ", "_")

        # Build new scene with just this child
        new_scene = deepcopy(root)
        # Remove all children from new scene
        for existing in list(new_scene):
            new_scene.remove(existing)
        # Add just this child
        new_scene.append(deepcopy(child))

        # Adjust totalTime to match element's endTime if available
        end_time = child.get("endTime")
        if end_time:
            new_scene.set("totalTime", end_time)

        # Generate XML
        xml_str = ET.tostring(new_scene, encoding="unicode", xml_declaration=False)
        full_xml = f"<?xml version='1.0' encoding='UTF-8' ?>\n{xml_str}\n"

        # Generate UUID filename
        xml_filename = f"{uuid.uuid4()}.xml"

        # Output amproj
        out_name = f"{i:03d}_{safe_label}.amproj"
        out_path = output_dir / out_name

        with zipfile.ZipFile(out_path, "w", zipfile.ZIP_DEFLATED) as zf:
            zf.writestr(xml_filename, full_xml)
            zf.writestr("manifest.txt", "")

        print(f"  [{i}] {out_name}  ({tag}, label={label!r})")

    print(f"\nOutput: {output_dir}/  ({len(children)} files)")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(__doc__.strip())
        sys.exit(1)
    out = sys.argv[2] if len(sys.argv) > 2 else None
    split_amproj(sys.argv[1], out)
