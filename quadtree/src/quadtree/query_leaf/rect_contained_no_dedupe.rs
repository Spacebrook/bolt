use super::super::*;

impl QuadTreeInner {
    #[inline(always)]
    // Safety: caller must ensure pointers are valid for `start..start+count`.
    pub(crate) unsafe fn query_rect_leaf_contained_raw_no_dedupe<F>(
        node_entity_values_ptr: *const u32,
        start: u32,
        count: usize,
        f: &mut F,
        stats: *mut QueryStats,
    ) where
        F: FnMut(u32),
    {
        #[cfg(not(feature = "query_stats"))]
        {
            let _ = stats;
        }
        let mut idx = start as usize;
        let end = idx + count;
        while idx < end {
            #[cfg(feature = "query_stats")]
            Self::bump_query_entity_ptr(stats);
            let value = *node_entity_values_ptr.add(idx);
            f(value);
            idx += 1;
        }
    }
}
