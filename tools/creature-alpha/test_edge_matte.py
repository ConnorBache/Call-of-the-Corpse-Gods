import importlib.util
from pathlib import Path
import sys
import unittest

import numpy as np

SPEC = importlib.util.spec_from_file_location("edge_matte", Path(__file__).with_name("edge_matte.py"))
edge = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = edge
SPEC.loader.exec_module(edge)


class EdgeMatteTests(unittest.TestCase):
    def fixture(self):
        src = np.zeros((13, 13, 4), dtype=np.uint8)
        src[:, :, 3] = 255
        src[3:10, 3:10, :3] = [240, 120, 48]
        cand = src.copy()
        cand[:, :, 3] = 0
        cand[3:10, 3:10, 3] = 255
        return src, cand

    def test_recovers_one_pixel_source_coverage_without_changing_interior(self):
        src, cand = self.fixture()
        src[6, 2, :3] = [60, 30, 12]
        out, report = edge.reconstruct_edge_matte(src, cand)
        self.assertEqual(int(out[6, 2, 3]), 64)
        self.assertTrue(np.all(np.abs(out[6, 2, :3].astype(int) - [240, 120, 48]) <= 1))
        np.testing.assert_array_equal(out[4:9, 4:9], cand[4:9, 4:9])
        self.assertEqual(report["restored_outer_rim_pixels"], 1)
        self.assertEqual(report["deleted_candidate_pixels"], 0)
        self.assertTrue(report["requires_manual_review"])

    def test_corrects_retained_boundary_without_eroding_support(self):
        src, cand = self.fixture()
        src[6, 3, :3] = [60, 30, 12]
        cand[6, 3, :3] = src[6, 3, :3]
        out, report = edge.reconstruct_edge_matte(src, cand)
        self.assertEqual(int(out[6, 3, 3]), 64)
        self.assertEqual(report["decontaminated_retained_edge_pixels"], 1)
        self.assertTrue(np.all(out[:, :, 3][cand[:, :, 3] > 0] > 0))

    def test_does_not_spread_into_remote_pixels(self):
        src, cand = self.fixture()
        src[6, 1, :3] = [60, 30, 12]
        out, _ = edge.reconstruct_edge_matte(src, cand)
        self.assertEqual(int(out[6, 1, 3]), 0)

    def test_preserves_neutral_outline_and_reports_ambiguity(self):
        src, cand = self.fixture()
        src[6, 3, :3] = [20, 20, 20]
        cand[6, 3, :3] = src[6, 3, :3]
        out, report = edge.reconstruct_edge_matte(src, cand)
        np.testing.assert_array_equal(out[6, 3], cand[6, 3])
        self.assertGreater(report["rejection_reasons"]["neutral-or-dark-outline-ambiguity"], 0)

    def test_preserves_authored_alpha_exactly(self):
        src, cand = self.fixture()
        src[1, 1, 3] = 127
        cand[1, 1, 3] = 127
        out, report = edge.reconstruct_edge_matte(src, cand)
        np.testing.assert_array_equal(out, src)
        self.assertEqual(report["bypass_reason"], "authored-alpha-preserved-exactly")

    def test_nonblack_matte_recomposition(self):
        src, cand = self.fixture()
        bg = np.array([20, 30, 40])
        outside = cand[:, :, 3] == 0
        src[:, :, :3][outside] = bg
        cand[:, :, :3][outside] = bg
        src[6, 2, :3] = [75, 53, 42]
        out, report = edge.reconstruct_edge_matte(src, cand, bg)
        self.assertGreater(int(out[6, 2, 3]), 0)
        a = out[6, 2, 3] / 255
        mixed = a * out[6, 2, :3] + (1 - a) * bg
        self.assertTrue(np.max(np.abs(mixed - src[6, 2, :3])) <= 1)
        self.assertEqual(report["changed_interior_pixels"], 0)

    def test_modified_candidate_rgb_is_rejected(self):
        src, cand = self.fixture()
        cand[6, 6, :3] = [10, 20, 30]
        with self.assertRaises(ValueError):
            edge.reconstruct_edge_matte(src, cand)

    def test_disagreement_is_not_averaged(self):
        src, cand = self.fixture()
        src[4:9, 4, :3] = [120, 60, 24]
        cand[4:9, 4, :3] = src[4:9, 4, :3]
        src[6, 2, :3] = [60, 30, 12]
        out, report = edge.reconstruct_edge_matte(src, cand)
        np.testing.assert_array_equal(out[6, 2], cand[6, 2])
        self.assertGreater(report["rejection_reasons"]["reference-colour-or-coverage-disagreement"], 0)

    def test_unstable_matte_bypasses(self):
        src, cand = self.fixture()
        src[0, :, :3] = [100, 50, 20]
        out, report = edge.reconstruct_edge_matte(src, cand)
        np.testing.assert_array_equal(out, cand)
        self.assertIn("matte-border", report["bypass_reason"])

    def test_nonbinary_candidate_is_rejected(self):
        src, cand = self.fixture()
        cand[1, 1, 3] = 1
        with self.assertRaises(ValueError):
            edge.reconstruct_edge_matte(src, cand)

    def test_deterministic_and_exact_rgba_reload(self):
        src, cand = self.fixture()
        src[6, 2, :3] = [60, 30, 12]
        a, ar = edge.reconstruct_edge_matte(src, cand)
        b, br = edge.reconstruct_edge_matte(src, cand)
        np.testing.assert_array_equal(a, b)
        self.assertEqual(ar, br)
        import io
        from PIL import Image
        stream = io.BytesIO()
        Image.fromarray(a, "RGBA").save(stream, format="PNG")
        stream.seek(0)
        reloaded = Image.open(stream)
        self.assertEqual(reloaded.mode, "RGBA")
        np.testing.assert_array_equal(np.array(reloaded), a)


if __name__ == "__main__":
    unittest.main()
