#!/usr/bin/env python3
"""Archive only small prototype artifacts, never dependencies or build outputs."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil

ROOT=Path(__file__).resolve().parent
def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('destination',type=Path)
    args=parser.parse_args()
    dest=args.destination.resolve()
    if dest==ROOT or ROOT in dest.parents:
        raise SystemExit('Archive destination must be outside the scratch source')
    excluded={'node_modules','target','__pycache__','.git'}
    copied=[]
    for path in sorted(ROOT.rglob('*')):
        relative=path.relative_to(ROOT)
        if not path.is_file() or excluded.intersection(relative.parts) or path.suffix in {'.db','.backup','.sqlite'}:
            continue
        if path.is_symlink():
            raise SystemExit('Unexpected symlink: '+str(relative))
        target=dest/relative
        target.parent.mkdir(parents=True,exist_ok=True)
        shutil.copy2(path,target)
        copied.append({'path':str(relative),'sha256':hashlib.sha256(target.read_bytes()).hexdigest(),'bytes':target.stat().st_size})
    manifest=dest/'capture-manifest.json'
    manifest.write_text(json.dumps({'source':str(ROOT),'scope':'Snapshot checksums; rerun logs may differ. No dependencies/build targets/databases.','files':copied},indent=2)+'\n')
    print(json.dumps({'destination':str(dest),'files':len(copied),'manifest':str(manifest)}))

if __name__=='__main__':
    main()

