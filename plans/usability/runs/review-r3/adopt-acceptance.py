"""Archive an independently verified report only after a named author review.

Usage: python3 -I adopt-acceptance.py <checkout> <base> <run> <author-review>
Never edits the original submitted work. Updates only its grading records.
"""
from pathlib import Path
import hashlib
import difflib
import json
import os
import shutil
import struct
import subprocess
import sys
import zlib

root, base = (Path(value).resolve() for value in sys.argv[1:3])
name = sys.argv[3]
assert name.startswith('r3-') and '/' not in name
review = Path(sys.argv[4]).resolve()
assert name in review.read_text(), 'The author review must identify this run'
a = base / ('acceptance-' + name)
original = base / name
out = root / 'plans/usability/runs' / name
assert out.is_dir(), 'Preserve the blind submission before acceptance adoption'
assert not (out / 'acceptance').exists(), 'Acceptance already archived; inspect rather than overwrite'
status_file = a / ('verifier-resume-status.txt' if (a / 'verifier-resume-status.txt').exists() else 'verifier-status.txt')
status = dict(line.split('=', 1) for line in status_file.read_text().splitlines())
assert status == {'exit_status': '0', 'original_source_unchanged': 'True'}
manifest = json.loads((a / 'original-source-manifest.json').read_text())
for relative, digest in manifest.items():
    path = original / 'work' / relative
    if digest.startswith('symlink:'):
        assert path.is_symlink() and os.readlink(path) == digest[len('symlink:'):], relative
        assert path.resolve().is_relative_to((original / 'work').resolve()), relative
    else:
        assert path.is_file() and hashlib.sha256(path.read_bytes()).hexdigest() == digest, relative
records = json.loads((a / 'original-record-manifest.json').read_text())
assert all((original / relative).is_file() and hashlib.sha256((original / relative).read_bytes()).hexdigest() == digest
           for relative, digest in records.items()), 'Original run records changed since verification'
for file in ('acceptance.md', 'acceptance.env'):
    assert not (out / file).exists() and not (original / file).exists(), file
    shutil.copy2(a / file, out / file)
    shutil.copy2(a / file, original / file)
archive = out / 'acceptance'
archive.mkdir()
shutil.copy2(review, archive / 'author-review.md')
# Retain test-only hooks separately from the immutable submitted source.
# The named author review must assess every existing-source difference.
for relative in manifest:
    submitted = original / 'work' / relative
    copied = a / 'work' / relative
    if not relative.endswith('.rs') or not submitted.is_file(): continue
    assert copied.is_file(), relative
    if copied.read_bytes() == submitted.read_bytes(): continue
    assert relative in review.read_text(), 'Author review must identify the changed source: ' + relative
    target = archive / 'test-entry-points' / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(copied, target)
    patch = ''.join(difflib.unified_diff(submitted.read_text().splitlines(True),
        copied.read_text().splitlines(True), fromfile='submitted/' + relative,
        tofile='acceptance-copy/' + relative))
    target.with_suffix(target.suffix + '.diff').write_text(patch)
for file in ('verifier-status.txt', 'verifier-resume-status.txt', 'original-source-manifest.json'):
    if (a / file).exists(): shutil.copy2(a / file, archive / file)
for path in (a / 'work').rglob('*.rs'):
    relative = path.relative_to(a / 'work')
    if 'target' in relative.parts or str(relative) in manifest: continue
    target = archive / 'added-tests' / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(path, target)
for path in (a / 'evidence').rglob('*'):
    if not path.is_file() or path.suffix == '.png': continue
    target = archive / 'evidence' / path.relative_to(a / 'evidence')
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(path, target)
provenance = {}
paths = [(path, 'work/' + str(path.relative_to(a / 'work'))) for path in (a / 'work').rglob('*.png')
         if 'target' not in path.relative_to(a / 'work').parts and str(path.relative_to(a / 'work')) not in manifest]
paths += [(path, 'evidence/' + str(path.relative_to(a / 'evidence'))) for path in (a / 'evidence').rglob('*.png')]
for path, relative in paths:
    data = path.read_bytes()
    assert data[:8] == b'\x89PNG\r\n\x1a\n'
    chunks = []; position = 8
    while position < len(data):
        length = struct.unpack('>I', data[position:position+4])[0]
        kind = data[position+4:position+8]; body = data[position+8:position+8+length]
        assert zlib.crc32(kind + body) == struct.unpack('>I', data[position+8+length:position+12+length])[0]
        chunks.append((kind, body)); position += length + 12
    raw = zlib.decompress(b''.join(body for kind, body in chunks if kind == b'IDAT'))
    compressed = zlib.compress(raw, 9)
    assert zlib.decompress(compressed) == raw
    encoded = bytearray(data[:8]); inserted = False
    for kind, body in chunks:
        if kind == b'IDAT':
            if inserted: continue
            body = compressed; inserted = True
        encoded += struct.pack('>I', len(body)) + kind + body + struct.pack('>I', zlib.crc32(kind + body))
    target = archive / 'captures' / relative
    target.parent.mkdir(parents=True, exist_ok=True); target.write_bytes(encoded)
    provenance[relative] = {'original_sha256': hashlib.sha256(data).hexdigest(), 'archived_sha256': hashlib.sha256(encoded).hexdigest(),
        'identical_decoded_IDAT_sha256': hashlib.sha256(raw).hexdigest(), 'operation': 'Lossless zlib recompression of IDAT; all other chunks unchanged.'}
if provenance: (archive / 'capture-provenance.json').write_text(json.dumps(provenance, indent=2) + '\n')
subprocess.run(['python3', str(root / 'plans/usability/summarize.py'), str(original)], cwd=root, check=True, stdout=subprocess.DEVNULL)
for file in ('summary.md', 'diagnostics.md'): shutil.copy2(original / file, out / file)
files = {str(path.relative_to(out)): hashlib.sha256(path.read_bytes()).hexdigest()
         for path in sorted(out.rglob('*')) if path.is_file() and path.name != 'snapshot.json'}
(out / 'snapshot.json').write_text(json.dumps(files, indent=2) + '\n')
print('Adopted after author review:', name)
