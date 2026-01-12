impl QuadTreeInner {
    #[allow(dead_code)]
    #[inline(always)]
    unsafe fn query_rect_leaf_contained_raw<F>(
        node_entity_packed_ptr: *const NodeEntityPacked,
        start: u32,
        count: usize,
        query_marks_ptr: *mut u32,
        tick: u32,
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
            let packed = *node_entity_packed_ptr.add(idx);
            let entity = packed.entity();
            let entity_idx = entity.index() as usize;
            if entity.has_dedupe() {
                let mark_ptr = query_marks_ptr.add(entity_idx);
                if *mark_ptr != tick {
                    *mark_ptr = tick;
                    f(packed.value());
                }
            } else {
                f(packed.value());
            }
            idx += 1;
        }
    }
}
