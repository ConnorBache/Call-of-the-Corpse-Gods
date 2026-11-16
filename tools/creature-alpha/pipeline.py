#!/usr/bin/env python3
"""Offline review-gated foreground segmentation; NEVER edits original artwork.
Audit writes candidates only. Build uses hash-bound approvals; uncertain assets
remain explicitly labelled frozen legacy references, never called cleaned.
"""
from __future__ import annotations
import argparse, hashlib, json
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage as ndi
ROOT=Path(__file__).resolve().parents[2]
ASSETS=ROOT/'crates/corpse-desktop/assets'
REVIEW=ROOT/'art-review'
CONFIG=Path(__file__).resolve().parent/'reviews.json'
BACKGROUNDS={'red':(135,18,27),'white':(255,255,255),'gray':(128,128,128),'black':(0,0,0)}
VERSION='reviewed-graphcut-v2'
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def write_json(path,data):
    path.parent.mkdir(parents=True,exist_ok=True);path.write_text(json.dumps(data,indent=2,sort_keys=True)+'\n')
def read_rgba(path):return np.array(Image.open(path).convert('RGBA'))
def save_rgba(path,rgba):
    path.parent.mkdir(parents=True,exist_ok=True);Image.fromarray(rgba.astype(np.uint8)).save(path)
def perimeter(rgb):return np.concatenate((rgb[0],rgb[-1],rgb[1:-1,0],rgb[1:-1,-1]))
def legacy_exterior(rgba):
    """Old 4-neighbor perimeter matting, NOT foreground segmentation.
    O(P) scan/propagation and memory. Used only to propose graph-cut seeds.
    """
    rgb=rgba[:,:,:3];p=perimeter(rgb);n=len(p);ordered=np.sort(p,axis=0)
    low=np.maximum(0,ordered[n//1000].astype(np.int16)-3)
    high=np.minimum(255,ordered[n-1-n//1000].astype(np.int16)+3)
    matches=np.all((rgb>=low)&(rgb<=high),axis=2)
    seeds=matches.copy();seeds[1:-1,1:-1]=False
    return ndi.binary_propagation(seeds,mask=matches),low,high

def legacy_reference(rgba):
    if np.any(rgba[:,:,3]<255):return rgba.copy()
    bg,_,_=legacy_exterior(rgba);out=rgba.copy();out[bg,3]=0;return out

def graphcut(rgba,protect_dark_core=True,trimap=None,iterations=5,background_pockets=()):
    """Global color-mixture + spatial min-cut, not a black color key.
    Distance transforms place seeds only; no morphology edits output geometry.
    Every strong detached feature is a seed; never keep largest component only.
    """
    import cv2
    if rgba.ndim!=3 or rgba.shape[2]!=4 or np.any(rgba[:,:,3]<255):raise ValueError('graphcut requires opaque RGBA; authored alpha must be preserved')
    rgb=rgba[:,:,:3];bg,low,high=legacy_exterior(rgba)
    median=np.median(perimeter(rgb),axis=0)
    diff=np.max(abs(rgb.astype(float)-median),axis=2)
    db=ndi.distance_transform_edt(bg);df=ndi.distance_transform_edt(~bg)
    seeds=np.full(bg.shape,cv2.GC_PR_BGD,np.uint8);seeds[~bg]=cv2.GC_PR_FGD
    if protect_dark_core:seeds[(~bg)&(df>=3)]=cv2.GC_FGD
    seeds[diff>=70]=cv2.GC_FGD;seeds[db>=3]=cv2.GC_BGD
    seeds[0,:]=0;seeds[-1,:]=0;seeds[:,0]=0;seeds[:,-1]=0
    if trimap is not None:
        if trimap.shape!=bg.shape or not set(np.unique(trimap)).issubset({0,128,255}):raise ValueError('invalid reviewed trimap')
        seeds[trimap==0]=0;seeds[trimap==255]=1
    # Asset-specific reviewer-labelled negative spaces. A bounded low-matte
    # component expands the seed, never a catalogue-wide black deletion rule.
    for x,y in background_pockets:
        eligible=rgb.max(2)<=12
        yy,xx=np.ogrid[:rgb.shape[0],:rgb.shape[1]]
        eligible &= (abs(xx-x)<=40)&(abs(yy-y)<=40)
        marker=np.zeros(bg.shape,bool);marker[y,x]=True
        if not eligible[y,x]:raise ValueError('reviewed BG seed no longer matches source')
        component=ndi.binary_propagation(marker,mask=eligible)
        uncertain=ndi.binary_dilation(component,iterations=3)&(diff<70)
        seeds[uncertain]=cv2.GC_PR_FGD;seeds[component]=cv2.GC_BGD
    initial=seeds.copy();cv2.setRNGSeed(0);cv2.setNumThreads(1)
    cv2.grabCut(np.ascontiguousarray(rgb[:,:,::-1]),seeds,None,np.zeros((1,65)),np.zeros((1,65)),iterations,cv2.GC_INIT_WITH_MASK)
    keep=(seeds==1)|(seeds==3);out=rgba.copy();out[~keep]=0;out[keep,3]=255
    edge=keep&ndi.binary_dilation(~keep);removed=(~bg)&(~keep)
    metrics={'retained_pixels':int(keep.sum()),'removed_from_legacy_mask':int(removed.sum()),
      'removed_rgb_max_gt32':int((removed&(rgb.max(2)>32)).sum()),
      'retained_dark_edge_pixels':int((edge&(rgb.max(2)<=12)).sum()),
      'restored_from_legacy_mask':int((keep&bg).sum()),
      'hard_foreground_lost':int(((initial==1)&~keep).sum()),
      'hard_background_retained':int(((initial==0)&keep).sum()),
      'foreground_components_8_connected':int(ndi.label(keep,np.ones((3,3)))[1]),
      'perimeter_channel_low':low.tolist(),'perimeter_channel_high':high.tolist(),'perimeter_median':median.tolist(),
      'rgb_changed_retained':0,'edge_color_correction':'none until separately reviewed local unmixing',
      'confidence':'unreviewed; semantic confidence is not inferred from graph energy'}
    return out,initial,metrics

def compare(name,original,legacy,candidate):
    directory=REVIEW/'comparisons'/name;directory.mkdir(parents=True,exist_ok=True)
    h,w=original.shape[:2];tw=min(w,420);th=round(h*tw/w)
    sheet=Image.new('RGB',(tw*3,(th+24)*4),(235,235,235));draw=ImageDraw.Draw(sheet)
    for row,(label,color) in enumerate(BACKGROUNDS.items()):
        for col,(tag,a) in enumerate([('original',original),('old runtime reference',legacy),('candidate: NOT approved',candidate)]):
            im=Image.new('RGBA',(w,h),(*color,255));im.alpha_composite(Image.fromarray(a))
            if col==2:im.save(directory/(label+'.png'))
            sheet.paste(im.resize((tw,th),Image.Resampling.NEAREST),(col*tw,row*(th+24)+24))
            draw.text((col*tw+4,row*(th+24)+4),f'{label} | {tag}',fill=(0,0,0))
    sheet.save(directory/'comparison.png')
    delta=np.zeros_like(original);delta[:,:,3]=255
    gone=(legacy[:,:,3]>0)&(candidate[:,:,3]==0);added=(legacy[:,:,3]==0)&(candidate[:,:,3]>0)
    delta[gone,:3]=[255,30,30];delta[added,:3]=[30,255,60]
    save_rgba(directory/'mask-difference.png',delta)

def audit(ids=None):
    import cv2,PIL,scipy
    decisions=json.loads(CONFIG.read_text()) if CONFIG.exists() else {'assets':{}}
    files=sorted((ASSETS/'creatures').glob('*.png'))
    if ids:files=[p for p in files if p.stem in ids]
    for path in files:
        name=path.stem;rgba=read_rgba(path)
        frozen=REVIEW/'legacy-reference'/(name+'.png')
        old=read_rgba(frozen) if frozen.exists() else legacy_reference(read_rgba(ASSETS/'creatures-distribution'/(name+'.webp')))
        spec=decisions.get('assets',{}).get(name,{})
        if spec.get('source_sha256') and spec['source_sha256']!=sha(path):raise ValueError(name+': reviewed source hash changed')
        if np.any(rgba[:,:,3]<255):
            result=rgba.copy();seeds=np.where(rgba[:,:,3]==0,0,1).astype(np.uint8)
            metrics={'confidence':'authored alpha preserved, not re-segmented','rgb_changed_retained':0};status='authored-alpha-preserved'
        else:
            trimap=None
            if spec.get('trimap'):
                mp=ROOT/spec['trimap']
                if sha(mp)!=spec['trimap_sha256']:raise ValueError('trimap hash mismatch')
                trimap=np.array(Image.open(mp).convert('L'))
            result,seeds,metrics=graphcut(rgba,spec.get('protect_dark_core',True),trimap,background_pockets=spec.get('reviewed_background_seeds',[]))
            binary=result.copy()
            save_rgba(REVIEW/'binary-candidates'/(name+'.png'),binary)
            maskpath=REVIEW/'trimaps'/(name+'-binary-alpha.png');maskpath.parent.mkdir(parents=True,exist_ok=True)
            Image.fromarray(binary[:,:,3]).save(maskpath)
            if spec.get('edge_reconstruction'):
                from edge_matte import reconstruct_edge_matte_evidence
                result,evidence,changed,uncertain=reconstruct_edge_matte_evidence(rgba,binary)
                metrics['edge_reconstruction']=evidence
                Image.fromarray(changed.astype(np.uint8)*255).save(REVIEW/'trimaps'/(name+'-correction.png'))
                Image.fromarray(uncertain.astype(np.uint8)*255).save(REVIEW/'trimaps'/(name+'-uncertain.png'))
                write_json(REVIEW/'reports'/(name+'-edge-evidence.json'),evidence)
            if 'preserve_uncertain_signal_above' in spec:
                level=spec['preserve_uncertain_signal_above']
                preserve=(result[:,:,3]==0)&(rgba[:,:,:3].max(2)>level)
                result[preserve]=rgba[preserve]
                metrics['conservative_source_pixels_restored']=int(preserve.sum())
                metrics['support_protection_note']='Source-signal guard is NOT semantic proof; unresolved weak/dark anatomy and matte remain manually queued.'
                metrics['deleted_source_rgb_max_above_guard']=int(((result[:,:,3]==0)&(rgba[:,:,:3].max(2)>level)).sum())
            status='candidate-needs-manual-review' 
        target=REVIEW/'candidates'/(name+'.png');save_rgba(target,result)
        mp=REVIEW/'trimaps'/(name+'-initial-labels.png');mp.parent.mkdir(parents=True,exist_ok=True);Image.fromarray(seeds).save(mp)
        compare(name,rgba,old,result)
        flags=['No silhouette approval: review dark anatomy, outlines, detached details, gaps and four backgrounds.']
        if 'zillennia' in name:flags.append('Intentional near-black body and thin tendrils require manual silhouette review; do not infer body from brightness.')
        if name=='vampire':flags=['Existing partial alpha may contain old matte; exact authored RGBA preserved, not a cleanup claim.']
        report={'id':name,'status':status,'source_sha256':sha(path),'candidate_sha256':sha(target),'seed_labels_sha256':sha(mp),
          'width':rgba.shape[1],'height':rgba.shape[0],'method':VERSION,'protect_dark_core':spec.get('protect_dark_core',True),
          'dependencies':{'opencv':cv2.__version__,'numpy':np.__version__,'pillow':PIL.__version__,'scipy':scipy.__version__},
          'graphcut_iterations':5,'rng_seed':0,'threads':1,'metrics':metrics,'manual_queue_reasons':flags}
        write_json(REVIEW/'reports'/(name+'.json'),report);print(name,status,metrics.get('removed_from_legacy_mask','existing-alpha'),flush=True)
    reports=[json.loads(p.read_text()) for p in sorted((REVIEW/'reports').glob('*.json')) if not p.name.endswith('-edge-evidence.json')]
    write_json(REVIEW/'audit.json',{'method':VERSION,'assets':reports})

def validate_approval(source,approved,spec):
    for field,actual in [('source_sha256',sha(source)),('approved_rgba_sha256',sha(approved))]:
        if spec.get(field)!=actual:raise ValueError('stale '+field)
    if not all(spec.get('checks',{}).get(x) for x in ['red','white','gray','black','anatomy','detached_details','dark_interior','source_overlay']):raise ValueError('incomplete review gate')
    if not spec.get('reviewer') or not spec.get('review_notes'):raise ValueError('review provenance required')
    original=read_rgba(source);result=read_rgba(approved)
    if original.shape!=result.shape:raise ValueError('approved dimensions differ from original')
    mask=ROOT/spec['reviewed_mask']
    if sha(mask)!=spec['reviewed_mask_sha256']:raise ValueError('stale reviewed mask')
    if not np.array_equal(np.array(Image.open(mask).convert('L')),result[:,:,3]):raise ValueError('approved alpha/mask mismatch')
    keep=result[:,:,3]>0
    if not np.any(~keep) or np.any(result[~keep]!=0):raise ValueError('clean output must have transparent-zero exterior')
    changed=np.any(original[:,:,:3]!=result[:,:,:3],axis=2)&keep
    partial=(result[:,:,3]>0)&(result[:,:,3]<255)
    correction=np.zeros(keep.shape,bool)
    if spec.get('rgb_correction_mask'):
        cp=ROOT/spec['rgb_correction_mask']
        if sha(cp)!=spec['rgb_correction_mask_sha256']:raise ValueError('stale correction evidence mask')
        correction=np.array(Image.open(cp).convert('L'))>0
        if correction.shape!=keep.shape:raise ValueError('correction mask dimensions')
        bp=ROOT/spec['reviewed_binary_mask']
        if sha(bp)!=spec['reviewed_binary_mask_sha256']:raise ValueError('stale binary silhouette')
        binary=np.array(Image.open(bp).convert('L'))>0
        glow=spec.get('glow_reconstruction')
        if glow:
            # Explicit, hash-bound exception for reviewed luminous matte, not
            # authority to recolor arbitrary pixels or widen ordinary edges.
            if glow.get('method')!='reviewed-black-matte-glow-v1' or glow.get('alpha_rule')!='max-source-channel-min-one' or glow.get('source_max_exclusive')!=80 or spec.get('matte_rgb')!=[0,0,0]:raise ValueError('unsupported glow reconstruction')
            pp=ROOT/glow['protected_anatomy']
            if sha(pp)!=glow['protected_anatomy_sha256']:raise ValueError('stale protected anatomy')
            protected=np.array(Image.open(pp).convert('L'))>0
            if protected.shape!=keep.shape:raise ValueError('protected anatomy dimensions')
            signal=original[:,:,:3].max(2)
            expected=binary&~protected&(signal<80)
            if not np.array_equal(correction,expected):raise ValueError('correction differs from reviewed glow region')
            expected_alpha=np.where(binary,255,0).astype(np.uint8)
            expected_alpha[expected]=np.maximum(1,signal[expected])
            if not np.array_equal(result[:,:,3],expected_alpha):raise ValueError('glow alpha is not source reconstruction')
            if not np.array_equal(keep,binary):raise ValueError('glow changed reviewed support')
        else:
            edge_band=ndi.binary_dilation(binary,structure=np.ones((3,3)))&ndi.binary_dilation(~binary,structure=np.ones((3,3)))
            if np.any(correction&~edge_band):raise ValueError('RGB correction outside one-pixel edge support')
        if np.any(binary&~keep):raise ValueError('edge correction deleted reviewed foreground')
        alpha=result[:,:,3].astype(float)/255
        matte=np.array(spec.get('matte_rgb',[0,0,0]),float)
        reconstructed=result[:,:,:3]*alpha[:,:,None]+matte*(1-alpha[:,:,None])
        if np.any(abs(reconstructed-original[:,:,:3])[correction]>2.01):raise ValueError('unmixing fails source-matte reconstruction')
    if np.any((changed|partial)&~correction):raise ValueError('unreviewed recolor or fractional alpha')
    if 'preserve_uncertain_signal_above' in spec and np.any((original[:,:,:3].max(2)>spec['preserve_uncertain_signal_above'])&~keep):raise ValueError('source support guard violated')

def frozen_legacy(name):
    provenance=json.loads((CONFIG.parent/'legacy-provenance.json').read_text())
    row=next(r for r in provenance['plates'] if r['id']==name)
    if sha(ASSETS/'creatures-distribution'/(name+'.webp'))!=row['source_webp_sha256']:raise ValueError('legacy input WebP changed')
    if sha(ASSETS/'creatures'/(name+'.png'))!=row['source_master_sha256']:raise ValueError('legacy input master changed')
    # The checked-in runtime PNG is also an exact legacy reference for an
    # unreviewed plate. This avoids a duplicated 100-MB authoritative tree.
    for path in [REVIEW/'legacy-reference'/(name+'.png'),ASSETS/'creatures-rgba'/(name+'.png')]:
        if path.exists():
            rgba=read_rgba(path)
            if hashlib.sha256(rgba.tobytes()).hexdigest()==row['rgba_sha256']:return rgba,row
    raise ValueError(name+': exact native legacy reference missing; regenerate with freeze_legacy_alpha example')

def build():
    decisions=json.loads(CONFIG.read_text());rows=[]
    summary=CONFIG.parent/'audit-summary.json'
    reports={r['id']:r for r in json.loads(summary.read_text())['assets']} if summary.exists() else {}
    originals=sorted((ASSETS/'creatures').glob('*.png'));assert originals
    for path in originals:
        name=path.stem;original=read_rgba(path);spec=decisions.get('assets',{}).get(name,{})
        report=reports.get(name,{})
        if np.any(original[:,:,3]<255):result=original;status='authored-alpha-preserved';reason='Verbatim authored RGBA; not claimed cleaned.'
        elif spec.get('status')=='approved':
            candidate=ROOT/spec['approved_rgba']
            validate_approval(path,candidate,spec)
            result=read_rgba(candidate);status=spec.get('promotion_kind','reviewed-segmentation');reason=spec['review_notes']
        else:
            result,_=frozen_legacy(name);status='legacy-reference-needs-manual-mask';reason=spec.get('review_notes','Original silhouette ambiguous/unreviewed; legacy extraction frozen, NOT fixed.')
        target=ASSETS/'creatures-rgba'/(name+'.png');save_rgba(target,result)
        rows.append({'id':name,'status':status,'reason':reason,'source_sha256':sha(path),'original_path':str(path.relative_to(ROOT)),
          'runtime_sha256':sha(target),'runtime_path':str(target.relative_to(ROOT)),'width':original.shape[1],'height':original.shape[0],
          'legacy_webp_sha256':sha(ASSETS/'creatures-distribution'/(name+'.webp')) if (ASSETS/'creatures-distribution'/(name+'.webp')).exists() else None,'legacy_rgba_sha256':hashlib.sha256(frozen_legacy(name)[0].tobytes()).hexdigest() if status.startswith('legacy') else None,'candidate_sha256':report.get('candidate_sha256'),
          'candidate_review_status':spec.get('status','not-reviewed'),'manual_queue_reasons':([reason]+spec.get('remaining_manual_work',[])) if ('needs-manual' in status) else []})
    counts={s:sum(p['status']==s for p in rows) for s in sorted({p['status'] for p in rows})}
    write_json(ASSETS/'creature-alpha-manifest.json',{'schema':1,'format':'RGBA PNG','runtime_keying':False,'source_masters_modified':False,'counts':counts,'plates':rows})
    queue=[p for p in rows if 'needs-manual' in p['status']]
    write_json(CONFIG.parent/'manual-queue.json',queue)
    write_json(REVIEW/'manual-queue.json',queue);print(counts)

def verify():
    manifest=json.loads((ASSETS/'creature-alpha-manifest.json').read_text());assert {p['id'] for p in manifest['plates']}=={p.stem for p in (ASSETS/'creatures').glob('*.png')}
    for row in manifest['plates']:
        source=ROOT/row['original_path'];runtime=ROOT/row['runtime_path']
        assert sha(source)==row['source_sha256'],row['id']+' original changed'
        assert sha(runtime)==row['runtime_sha256'],row['id']+' runtime changed'
        im=Image.open(runtime);assert im.mode=='RGBA';assert im.size==(row['width'],row['height'])
        if row['status']=='authored-alpha-preserved':assert np.array_equal(read_rgba(source),read_rgba(runtime))
        if row['status'].startswith('legacy'):assert hashlib.sha256(read_rgba(runtime).tobytes()).hexdigest()==row['legacy_rgba_sha256']
        if row['status'].startswith('reviewed-'):
            a=read_rgba(runtime);assert (a[:,:,3]==0).any();assert np.all(a[a[:,:,3]==0]==0)
            spec=json.loads(CONFIG.read_text())['assets'][row['id']];validate_approval(source,runtime,spec)
    print(f"Verified {len(manifest['plates'])} source/runtime hashes, RGBA dimensions and authored-alpha identity.")
if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('command',choices=['audit','build','verify']);ap.add_argument('--only',nargs='+');a=ap.parse_args()
    {'audit':lambda:audit(a.only),'build':build,'verify':verify}[a.command]()
