#!/usr/bin/env python3
"""O(model source bytes): ensure omitted snapshot-root list tracks serde schema."""
import pathlib,re
root=pathlib.Path(__file__).resolve().parents[1]
s=(root/'crates/corpse-core/src/model_generated.rs').read_text().split('pub struct Run {',1)[1].split('\n}',1)[0]
keys=[];start=0
for m in re.finditer(r'pub (\w+):[^\n]+',s):
 attrs=s[start:m.start()];start=m.end()
 if 'skip_serializing_if' not in attrs:continue
 rename=re.search(r'rename\s*=\s*"([^"]+)"',attrs);key=rename.group(1) if rename else m.group(1)
 if key not in ['history','replay','hasteSnapshot','logArchive','runId','replaySnapshotFrom','fireboltTravelFrom','amuletRulesFrom','potionRulesFrom','ringRulesFrom','gasRulesFrom','commandmentRulesFrom','avatarStarterPolicy','emberRulesFrom','steamRulesFrom','executeRulesFrom','beckoningRulesFrom','ichorFireRulesFrom','ragelightRulesFrom','balanceRulesFrom']:keys.append(key)
t=(root/'crates/corpse-core/src/temporal.rs').read_text();body=t.split('const SNAPSHOT_OPTIONAL_ROOTS: &[&str] = &[',1)[1].split('];',1)[0]
assert re.findall(r'"([^"]+)"',body)==keys,(keys,body)
print('snapshot optional root schema checked:',len(keys))
