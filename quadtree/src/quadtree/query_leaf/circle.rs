impl QuadTreeInner {
    #[inline(always)]
    // Safety: caller must ensure pointers are valid for `start..start+count` and `query_marks_ptr`
    // is at least `max_entity_index + 1` long; `stats` may be null when query_stats is disabled.
    unsafe fn query_circle_leaf<F>(
        node_entity_packed_ptr: *const NodeEntityPacked,
        start: u32,
        count: usize,
        query_marks_ptr: *mut u32,
        circle_data_ptr: *const CircleData,
        _query_extent: RectExtent,
        query_kind: QueryKind,
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
            let circle = *circle_data_ptr.add(entity_idx);
            let hit = match query_kind {
                QueryKind::Rect {
                    x,
                    y,
                    half_w,
                    half_h,
                } => circle_rect_raw(
                    circle.x,
                    circle.y,
                    circle.radius,
                    circle.radius_sq,
                    x,
                    y,
                    half_w,
                    half_h,
                ),
                QueryKind::Circle { x, y, radius, radius_sq: _ } => {
                    circle_circle_raw(x, y, radius, circle.x, circle.y, circle.radius)
                }
            };
            if hit {
                if entity.has_dedupe() {
                    let mark_ptr = query_marks_ptr.add(entity_idx);
                    *mark_ptr = tick;
                }
                f(packed.value());
            }
            idx += 1;
        }
    }

    #[inline(always)]
    // Safety: caller must ensure pointers are valid for `start..start+count`.
    unsafe fn query_circle_leaf_no_dedupe<F>(
        node_entity_packed_ptr: *const NodeEntityPacked,
        start: u32,
        count: usize,
        circle_data_ptr: *const CircleData,
        _query_extent: RectExtent,
        query_kind: QueryKind,
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
            let entity_idx = packed.entity().index() as usize;
            let circle = *circle_data_ptr.add(entity_idx);
            let hit = match query_kind {
                QueryKind::Rect {
                    x,
                    y,
                    half_w,
                    half_h,
                } => circle_rect_raw(
                    circle.x,
                    circle.y,
                    circle.radius,
                    circle.radius_sq,
                    x,
                    y,
                    half_w,
                    half_h,
                ),
                QueryKind::Circle { x, y, radius, radius_sq: _ } => {
                    circle_circle_raw(x, y, radius, circle.x, circle.y, circle.radius)
                }
            };
            if hit {
                f(packed.value());
            }
            idx += 1;
        }
    }
}
