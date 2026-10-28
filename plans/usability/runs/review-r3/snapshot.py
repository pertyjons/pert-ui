"""Preserve a terminal usability run, with lossless PNG storage.

Usage: python3 snapshot.py <checkout> <run-base> <run-name>
Never copies CLI homes, credentials, transcripts, or build directories.
"""
from pathlib import Path
import hashlib
import json
import shutil
import struct
import sys
import zlib

root,base=map(lambda value:Path(value).resolve(),sys.argv[1:3])
name=sys.argv[3]
assert name.startswith('r3-') and '/' not in name
run=base/name
conditions=dict(line.split('=',1) for line in (run/'conditions.env').read_text().splitlines() if '=' in line)
if not conditions.get('ended') or not conditions.get('outcome'):
    raise SystemExit('Run is not terminal; refusing to snapshot changing source.')
for link in (run/'work').rglob('*'):
    if link.is_symlink() and (Path(link.readlink()).is_absolute() or not link.resolve().is_relative_to((run/'work').resolve())):
        raise SystemExit(f'Nonportable source symlink; inspect before snapshot: {link.relative_to(run)}')
out=root/'plans/usability/runs'/name
if out.exists():
    raise SystemExit('Snapshot exists; inspect it rather than overwrite.')
out.mkdir()
for file in ('conditions.env','summary.md','diagnostics.md'):
    shutil.copy2(run/file,out/file)
if (run/'work/FRICTION.md').exists():
    shutil.copy2(run/'work/FRICTION.md',out/'FRICTION.md')

def ignore(directory,names):
    excluded={'target','.git','.claude','.codex','Cargo.lock','FRICTION.md','docs-text'}
    # Keep generated kit-documentation dumps in the raw run only. Preserve
    # application-authored docs: only a directory entirely of rustdoc-page
    # conversions is excluded here.
    docs=Path(directory)/'docs'
    if 'docs' in names and docs.is_dir():
        files=list(docs.rglob('*'))
        if files and all(p.is_file() and p.suffix=='.md' and 'Copy item path' in p.read_text()[:200] for p in files):
            excluded.add('docs')
    # Top-level PNGs are the tasks' generated captures. Keep assets in nested
    # source directories in the submitted project, since code may include them.
    if Path(directory)==run/'work':
        excluded.update(name for name in names if name.endswith('.png'))
    return excluded.intersection(names)

shutil.copytree(run/'work',out/'submission',ignore=ignore,symlinks=True)
provenance={}
for path in sorted((run/'work').glob('*.png')):
    original=path.read_bytes()
    assert original[:8]==b'\x89PNG\r\n\x1a\n'
    chunks=[]; streams=[]; pos=8
    while pos<len(original):
        length=struct.unpack('>I',original[pos:pos+4])[0]
        kind=original[pos+4:pos+8];data=original[pos+8:pos+8+length]
        crc=struct.unpack('>I',original[pos+8+length:pos+12+length])[0]
        assert zlib.crc32(kind+data)==crc
        chunks.append((kind,data));pos+=length+12
        if kind==b'IDAT':streams.append(data)
    raw=zlib.decompress(b''.join(streams));compressed=zlib.compress(raw,9)
    assert zlib.decompress(compressed)==raw
    encoded=bytearray(original[:8]);inserted=False
    for kind,data in chunks:
        if kind==b'IDAT':
            if inserted:continue
            data=compressed;inserted=True
        encoded+=struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data))
    captures=out/'captures';captures.mkdir(exist_ok=True)
    (captures/path.name).write_bytes(encoded)
    provenance[path.name]={'original_sha256':hashlib.sha256(original).hexdigest(),
                           'archived_sha256':hashlib.sha256(encoded).hexdigest(),
                           'identical_decoded_IDAT_sha256':hashlib.sha256(raw).hexdigest(),
                           'operation':'Lossless zlib recompression of IDAT; all other chunks unchanged.'}
if provenance:
    (out/'capture-provenance.json').write_text(json.dumps(provenance,indent=2)+'\n')
manifest={str(p.relative_to(out)):hashlib.sha256(p.read_bytes()).hexdigest() for p in out.rglob('*') if p.is_file()}
(out/'snapshot.json').write_text(json.dumps(manifest,indent=2)+'\n')
print('Preserved',name,flush=True)
