impl QuadTreeInner {
    #[allow(dead_code)]
    #[inline(always)]
    unsafe fn query_rect_leaf_raw<F>(
        node_entity_packed_ptr: *const NodeEntityPacked,
        start: u32,
        count: usize,
        query_marks_ptr: *mut u32,
        q_min_x: f32,
        q_min_y: f32,
        q_max_x: f32,
        q_max_y: f32,
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
                if *mark_ptr == tick {
                    idx += 1;
                    continue;
                }
            }
            if packed.max_x <= q_min_x || q_max_x <= packed.min_x {
                idx += 1;
                continue;
            }
            if packed.max_y <= q_min_y || q_max_y <= packed.min_y {
                idx += 1;
                continue;
            }
            if entity.has_dedupe() {
                let mark_ptr = query_marks_ptr.add(entity_idx);
                *mark_ptr = tick;
            }
            f(packed.value());
            idx += 1;
        }
    }
}
