#!/usr/bin/env python3
"""Regression checks for five manually reviewed Bestiary alpha repairs.
Run from any directory: python tools/creature-alpha/test_targeted_repairs.py
Uses exact source pixels, manifests, and hash-bound masks; no segmentation runs.
"""
import copy,hashlib,json,tempfile,unittest
from pathlib import Path
import numpy as np
from PIL import Image
import pipeline as p
IDS=['horn-6','ursa-minor','samara','horn-7','valta']
class TargetedPortraitRepairs(unittest.TestCase):
 def setUp(self):self.specs=json.loads(p.CONFIG.read_text())['assets']
 def test_all_five_review_gates(self):
  for name in IDS:
   spec=self.specs[name];p.validate_approval(p.ASSETS/'creatures'/f'{name}.png',p.ROOT/spec['approved_rgba'],spec)
 def test_binary_repairs_preserve_source_rgb_and_regression_points(self):
  points={'horn-6':[(380,720),(650,790),(720,840),(960,810)],'ursa-minor':[(807,625),(850,690),(890,735),(935,725),(400,815),(450,835),(510,845)]}
  for name in IDS[:-1]:
   a=p.read_rgba(p.ASSETS/'creatures-rgba'/f'{name}.png');s=p.read_rgba(p.ASSETS/'creatures'/f'{name}.png');keep=a[:,:,3]>0
   self.assertTrue(set(np.unique(a[:,:,3])).issubset({0,255}));self.assertTrue(np.array_equal(a[keep,:3],s[keep,:3]))
   for x,y in points.get(name,[]):self.assertEqual(int(a[y,x,3]),255,(name,x,y))
   if name in ['samara','horn-7']:
    evidence=json.loads((p.CONFIG.parent/'reviewed'/f'{name}-mask-review.json').read_text())
    for r in evidence['reviewed_regions']:
     x,y=r['seed'];self.assertEqual(int(a[y,x,3]),0,(name,x,y))
 def test_compact_alpha_and_dimensions_exact_for_all_84(self):
  rows=json.loads((p.ASSETS/'creature-runtime-manifest.json').read_text())['plates'];self.assertEqual(len(rows),84)
  for r in rows:
   src=p.ASSETS/'creatures-rgba'/r['source_file'];web=p.ASSETS/'creatures-runtime'/r['encoded_file']
   self.assertEqual(p.sha(src),r['source_sha256']);self.assertEqual(p.sha(web),r['encoded_sha256'])
   a=p.read_rgba(src);b=p.read_rgba(web);self.assertEqual(a.shape,b.shape);self.assertTrue(np.array_equal(a[:,:,3],b[:,:,3]),r['id'])
 def test_valta_preserves_glow_support_and_black_source(self):
  spec=self.specs['valta'];a=p.read_rgba(p.ASSETS/'creatures-rgba/valta.png');s=p.read_rgba(p.ASSETS/'creatures/valta.png')
  binary=np.array(Image.open(p.ROOT/spec['reviewed_binary_mask']))>0
  correction=np.array(Image.open(p.ROOT/spec['rgb_correction_mask']))>0
  self.assertTrue(np.array_equal(a[:,:,3]>0,binary));self.assertEqual(int(a[750,550,3]),255)
  reconstruction=a[:,:,:3].astype(float)*(a[:,:,3,None]/255)
  self.assertLessEqual(float(abs(reconstruction-s[:,:,:3])[correction].max()),1.)
 def test_valta_rejects_recolor_outside_reviewed_glow(self):
  spec=copy.deepcopy(self.specs['valta']);a=p.read_rgba(p.ROOT/spec['approved_rgba']);a[600,500,0]^=16
  with tempfile.TemporaryDirectory() as tmp:
   target=Path(tmp)/'tampered.png';Image.fromarray(a).save(target);spec['approved_rgba_sha256']=p.sha(target)
   with self.assertRaises(ValueError):p.validate_approval(p.ASSETS/'creatures/valta.png',target,spec)
if __name__=='__main__':unittest.main(verbosity=2)
