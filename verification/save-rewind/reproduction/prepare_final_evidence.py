"""Collect verified release evidence without modifying source. O(evidence bytes)."""
from pathlib import Path
import json,shutil,zipfile,hashlib
r=Path(__file__).resolve().parents[1];extra=r/'release-extra/verification/save-rewind';extra.mkdir(parents=True,exist_ok=True)
summary=json.loads((r/'reports/stage2/full-test-summary.json').read_text());assert summary['all_targets_passed']
for name in ['quiet-summary.json','quiet-summary.txt','full-test-summary.json','final-code-provenance.json','windows-build-profile.json','windows-package.log','doc-tests.log']:
 shutil.copy2(r/'reports/stage2'/name,extra/name)
shutil.copy2(r/'reports/stage2/cold-repeat/summary.json',extra/'cold-repeat-summary.json')
shutil.copy2(r/'diagnostic/summary.json',extra/'flush-diagnostic-summary.json')
shutil.copy2(r/'diagnostic/RESULT.md',extra/'flush-diagnostic.md')
shutil.copytree(r/'reports/stage2/full-tests',extra/'full-tests',dirs_exist_ok=True)
shutil.copytree(r/'qa',extra/'reproduction',dirs_exist_ok=True,ignore=shutil.ignore_patterns('__pycache__','PAUSE*','CANCEL*'))
# Large exact saved-byte examples live in separate evidence ZIP, not duplicated in full source.
p=r/'package/save-rewind-measurements-pending-gates.zip'
with zipfile.ZipFile(p,'w',zipfile.ZIP_DEFLATED,compresslevel=6)as z:
 for base,prefix in [(r/'reports/quiet-stage2','normal-release'),(r/'reports/stage2/cold-repeat','cold-repeat'),(extra,'verification')]:
  for f in sorted(base.rglob('*')):
   if f.is_file():z.write(f,prefix+'/'+f.relative_to(base).as_posix())
 for f in sorted((r/'diagnostic').rglob('*')):
  rel=f.relative_to(r/'diagnostic')
  if f.is_file()and not any(x.startswith('target-') for x in rel.parts):z.write(f,'diagnostic/'+rel.as_posix())
 z.writestr('README.txt','Final save/rewind coarse journal verification evidence. Normal command+encode is not durable flush/end-to-end/Windows runtime. Original cold2.66s outlier retained, not reproduced in bounded repeat; its cause unknown. Diagnostic attributed its own267ms sample primarily to encoding wall interval, not oldoutliercause. Full final test summary and source/build provenance included. Raw exact saved-byte files preserved for22 cross-binary scenarios. No save format or durability weakening.\n')
with zipfile.ZipFile(p)as z:assert z.testzip()is None
print(json.dumps({'path':str(p),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()},indent=2))
