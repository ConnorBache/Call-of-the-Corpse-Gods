//! Exact nonzero row runs for cached ASCII coverage. No tint or framebuffer cache.

pub(super) struct AsciiMask {
    pub(super) coverage: Vec<u8>,
    pub(super) row_offsets: Vec<u32>,
    pub(super) runs: Vec<(i32, i32)>,
    /// Exclusive last covered row, computed once for bottom-aligned ground ink.
    pub(super) ink_bottom: usize,
}

impl AsciiMask {
    /// Once per cache insertion: O(W*H) time, O(H+R) extra storage for R maximal
    /// nonzero runs. Coverage remains byte-for-byte unchanged for every opacity.
    pub(super) fn new(coverage: Vec<u8>, width: usize, height: usize) -> Self {
        assert!(width > 0 && width <= i32::MAX as usize);
        assert_eq!(coverage.len(), width.checked_mul(height).unwrap());
        let mut row_offsets = Vec::with_capacity(height + 1);
        let mut runs = Vec::new();
        row_offsets.push(0);
        for row in coverage.chunks_exact(width) {
            let mut x = 0;
            while x < width {
                if row[x] == 0 { x += 1; continue; }
                let start = x;
                while x < width && row[x] != 0 { x += 1; }
                runs.push((start as i32, x as i32));
            }
            row_offsets.push(u32::try_from(runs.len()).expect("ASCII run count exceeds u32"));
        }
        let ink_bottom = row_offsets.windows(2).rposition(|r| r[0] != r[1]).map_or(0, |y| y + 1);
        Self { coverage, row_offsets, runs, ink_bottom }
    }

    /// O(log R_row + intersecting runs), preserving left-to-right order without
    /// scanning transparent coverage. Called after native/shear/tile clipping.
    pub(super) fn row_runs(&self, row: usize, left: i32, right: i32)
        -> impl Iterator<Item = (i32, i32)> + '_ {
        let runs = &self.runs[self.row_offsets[row] as usize..self.row_offsets[row + 1] as usize];
        let first = runs.partition_point(|&(_, end)| end <= left);
        runs[first..].iter().copied().take_while(move |&(start, _)| left < right && start < right)
            .map(move |(start, end)| (start.max(left), end.min(right)))
    }
}
