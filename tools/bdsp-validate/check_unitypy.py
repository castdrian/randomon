import sys
import types
from pathlib import Path


def stub_tkinter():
    tkinter = types.ModuleType("tkinter")
    messagebox = types.ModuleType("tkinter.messagebox")
    messagebox.NO = "no"
    tkinter.messagebox = messagebox
    sys.modules.setdefault("tkinter", tkinter)
    sys.modules.setdefault("tkinter.messagebox", messagebox)


stub_tkinter()

import UnityPy


def decoded_objects(path):
    environment = UnityPy.load(str(path))
    objects = {}
    for item in environment.objects:
        if item.serialized_type is not None and item.serialized_type.nodes:
            objects[item.path_id] = item.read_typetree()
        else:
            objects[item.path_id] = bytes(item.get_raw_data())
    return objects


def compare(original_path, edited_path):
    original = decoded_objects(original_path)
    edited = decoded_objects(edited_path)
    if original.keys() != edited.keys():
        raise SystemExit(f"{edited_path}: object set differs")
    changed = [path_id for path_id in original if original[path_id] != edited[path_id]]
    return len(original), changed


def main():
    data_root = Path(sys.argv[1])
    romfs_root = Path(sys.argv[2])
    edited_files = sorted(path for path in romfs_root.rglob("*") if path.is_file())
    if not edited_files:
        raise SystemExit("no edited files found")
    for edited_path in edited_files:
        relative = edited_path.relative_to(romfs_root)
        total, changed = compare(data_root / relative, edited_path)
        print(f"{relative}: {total} objects load in UnityPy, {len(changed)} changed: {changed[:12]}")


main()
