import importlib.util, pathlib, tempfile, unittest
import numpy as np
from PIL import Image
PATH=pathlib.Path(__file__).resolve().parents[1]/'pipeline.py'
spec=importlib.util.spec_from_file_location('pipeline',PATH);p=importlib.util.module_from_spec(spec);spec.loader.exec_module(p)
class SegmentationTests(unittest.TestCase):
    def test_explicit_mask_preserves_black_anatomy_single_pixel_tendrils_and_detached_sparks(self):
        a=np.zeros((48,48,4),np.uint8);a[:,:,3]=255;a[12:36,12:36,:3]=[180,80,30]
        # Black interior, black anatomy touching black background, one-pixel diagonal.
        a[20:28,20:28,:3]=0
        trimap=np.full((48,48),128,np.uint8)
        trimap[20:28,20:28]=255
        trimap[24,36:43]=255
        for n in range(7):trimap[35+n,35+n]=255
        for y,x in [(6,6),(8,40)]:a[y,x,:3]=[255,100,20]
        result,_,stats=p.graphcut(a,trimap=trimap)
        self.assertTrue(np.all(result[trimap==255,3]==255))
        self.assertTrue(np.array_equal(result[trimap==255,:3],a[trimap==255,:3]))
        self.assertEqual(result[6,6,3],255);self.assertEqual(result[8,40,3],255)
        self.assertEqual(stats['hard_foreground_lost'],0)
        self.assertTrue(set(np.unique(result[:,:,3])).issubset({0,255}))
        self.assertTrue(np.all(result[result[:,:,3]==0]==0))
    def test_authored_rgba_is_never_rekeyed_by_legacy_import(self):
        a=np.array([[[0,0,0,0],[0,0,0,255],[1,2,3,127]]],np.uint8)
        self.assertTrue(np.array_equal(p.legacy_reference(a),a))
    def test_dark_subject_below_gray_matte_is_not_a_global_black_key(self):
        a=np.full((40,40,4),255,np.uint8);a[:,:,:3]=[16,16,17];a[12:28,12:28,:3]=[2,1,3]
        result,_,_=p.graphcut(a)
        self.assertTrue(np.all(result[15:25,15:25,3]==255))
        self.assertTrue(np.all(result[15:25,15:25,:3]==[2,1,3]))
        self.assertEqual(result[0,0,3],0)
    def test_graphcut_rejects_any_authored_partial_alpha(self):
        for alpha in [0,1,127,254]:
            a=np.full((16,16,4),255,np.uint8);a[5,5,3]=alpha
            with self.assertRaisesRegex(ValueError,'authored alpha'):p.graphcut(a)
    def test_reviewed_partial_support_and_known_weak_flames(self):
        for name in ['phoenix','horn-1','river-drake']:
            path=p.CONFIG.parent/'reviewed'/(name+'.png')
            if not path.exists():self.skipTest('reviewed asset not present')
            original=p.read_rgba(p.ASSETS/'creatures'/(name+'.png'));approved=p.read_rgba(path)
            self.assertFalse(np.any((original[:,:,:3].max(2)>32)&(approved[:,:,3]==0)),name)
        phoenix=p.read_rgba(p.CONFIG.parent/'reviewed'/'phoenix.png')
        original=p.read_rgba(p.ASSETS/'creatures'/'phoenix.png')
        for x,y in [(582,271),(158,32),(197,35),(574,390)]:
            self.assertEqual(phoenix[y,x,3],255)
            self.assertTrue(np.array_equal(phoenix[y,x],original[y,x]))
    def test_invalid_trimap_rejected(self):
        a=np.zeros((16,16,4),np.uint8);a[:,:,3]=255
        with self.assertRaisesRegex(ValueError,'invalid reviewed trimap'):p.graphcut(a,trimap=np.full((16,16),42,np.uint8))
    def test_unapproved_or_stale_output_cannot_promote(self):
        with tempfile.TemporaryDirectory() as t:
            f=pathlib.Path(t)/'sample.png';Image.new('RGBA',(4,4)).save(f)
            with self.assertRaisesRegex(ValueError,'stale source'):p.validate_approval(f,f,{})
            with self.assertRaisesRegex(ValueError,'incomplete review'):p.validate_approval(f,f,{'source_sha256':p.sha(f),'approved_rgba_sha256':p.sha(f)})
    def test_approved_file_cannot_resize_recolor_or_smear_alpha(self):
        with tempfile.TemporaryDirectory() as t:
            oldroot=p.ROOT;p.ROOT=pathlib.Path(t)
            try:
                src=p.ROOT/'original.png';out=p.ROOT/'approved.png';mask=p.ROOT/'alpha.png'
                original=np.zeros((8,8,4),np.uint8);original[:,:,3]=255;original[2:6,2:6,:3]=[120,40,20]
                clean=original.copy();clean[:2]=0;clean[6:]=0;clean[:,:2]=0;clean[:,6:]=0
                p.save_rgba(src,original)
                def config(a):
                    p.save_rgba(out,a);Image.fromarray(a[:,:,3]).save(mask)
                    return {'source_sha256':p.sha(src),'approved_rgba_sha256':p.sha(out),
                      'reviewed_mask':'alpha.png','reviewed_mask_sha256':p.sha(mask),
                      'reviewer':'unit fixture','review_notes':'known shape','checks':dict.fromkeys(['red','white','gray','black','anatomy','detached_details','dark_interior','source_overlay'],True)}
                p.validate_approval(src,out,config(clean))
                with self.assertRaisesRegex(ValueError,'dimensions'):p.validate_approval(src,out,config(clean[:7]))
                bad=clean.copy();bad[3,3,0]+=10
                with self.assertRaisesRegex(ValueError,'recolor'):p.validate_approval(src,out,config(bad))
                bad=clean.copy();bad[3,3,3]=127
                with self.assertRaisesRegex(ValueError,'fractional alpha'):p.validate_approval(src,out,config(bad))
            finally:p.ROOT=oldroot
    def test_rejected_dark_anatomy_candidate_is_not_packaged(self):
        manifest=p.ASSETS/'creature-alpha-manifest.json'
        if not manifest.exists():self.skipTest('runtime build not present')
        import json,hashlib
        row=next(x for x in json.loads(manifest.read_text())['plates'] if x['id']=='horn-6')
        self.assertEqual(row['status'],'legacy-reference-needs-manual-mask')
        # Raw RGBA equality, not merely PNG file metadata or a review label.
        rgba,provenance=p.frozen_legacy('horn-6')
        runtime=p.read_rgba(p.ROOT/row['runtime_path'])
        self.assertEqual(hashlib.sha256(runtime.tobytes()).hexdigest(),provenance['rgba_sha256'])
    def test_deterministic_seeded_graph_cut(self):
        a=np.zeros((32,32,4),np.uint8);a[:,:,3]=255;a[8:24,8:24,:3]=[128,45,20]
        one,_,_=p.graphcut(a);two,_,_=p.graphcut(a)
        self.assertTrue(np.array_equal(one,two))
if __name__=='__main__':unittest.main()
