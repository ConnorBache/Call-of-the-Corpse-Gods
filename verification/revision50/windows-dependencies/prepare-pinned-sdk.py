#!/usr/bin/env python3
"""Workspace-only pinned official dependency restoration. No game build."""
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import hashlib,json,subprocess,tarfile,time,urllib.parse
root=Path(__file__).resolve().parent
repo=root.parent/'r50-work'
entries=json.loads((repo/'verification/integration-r39/windows-build/debian-package-manifest.json').read_text())
sdl=json.loads((repo/'verification/integration-r39/windows-build/sdl-manifest.json').read_text())
entries.append({'package':'SDL2-devel','version':'2.32.10',**sdl})
(root/'receipts/pinned-archives-manifest.json').write_text(json.dumps(entries,indent=2)+'\n')

def fetch(item):
    url=item['url']; dest=root/'downloads'/urllib.parse.urlparse(url).path.rsplit('/',1)[1]
    receipt={'package':item['package'],'version':item['version'],'url':url,'file':str(dest)}
    if not dest.exists():
        partial=dest.with_name(dest.name+'.partial')
        result=subprocess.run(['curl','--fail','--location','--silent','--show-error','--max-time','300','--output',str(partial),'--dump-header',str(root/'receipts'/(dest.name+'.http-headers.txt')),url],capture_output=True,text=True)
        if result.returncode: raise RuntimeError(f'{url}: curl {result.returncode}: {result.stderr}')
        digest=hashlib.sha256(partial.read_bytes()).hexdigest()
        if digest!=item['sha256'] or partial.stat().st_size!=item['bytes']: raise RuntimeError(f'Hash/size mismatch: {url}: {digest}, {partial.stat().st_size}')
        partial.rename(dest)
    digest=hashlib.sha256(dest.read_bytes()).hexdigest()
    if digest!=item['sha256'] or dest.stat().st_size!=item['bytes']: raise RuntimeError(f'Hash/size mismatch: {url}: {digest}, {dest.stat().st_size}')
    receipt.update(sha256=digest,bytes=dest.stat().st_size,verified=True)
    (root/'receipts'/(dest.name+'.json')).write_text(json.dumps(receipt,indent=2)+'\n')
    print(f"VERIFIED {item['package']} {digest} ({receipt['bytes']} bytes)",flush=True)
    return dest
with ThreadPoolExecutor(max_workers=3) as pool:
    files=list(pool.map(fetch,entries))
for dest in files:
    if dest.suffix=='.deb':
        subprocess.run(['dpkg-deb','--extract',str(dest),str(root/'sysroot')],check=True)
    else:
        with tarfile.open(dest) as archive:
            for member in archive.getmembers():
                p=Path(member.name)
                if p.is_absolute() or '..' in p.parts: raise RuntimeError(f'Unsafe archive path {member.name}')
            archive.extractall(root,filter='data')
    print(f'EXTRACTED {dest.name}',flush=True)
print('All pinned SDK archives verified and extracted without package maintainer scripts.',flush=True)
