import sys,json,hashlib
for line in sys.stdin.buffer:
 rev,raw=line.rstrip(b'\n').split(b'\t',1)
 print(json.dumps({'revision':int(rev),'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest()}),flush=True)
