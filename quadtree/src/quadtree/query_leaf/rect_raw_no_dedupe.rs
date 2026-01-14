impl QuadTreeInner {
    #[inline(always)]
    // Safety: caller must ensure pointers are valid for `start..start+count`.
    unsafe fn query_rect_leaf_raw_no_dedupe<F>(
        node_entity_min_x_ptr: *const f32,
        node_entity_min_y_ptr: *const f32,
        node_entity_max_x_ptr: *const f32,
        node_entity_max_y_ptr: *const f32,
        node_entity_values_ptr: *const u32,
        start: u32,
        count: usize,
        use_avx2: bool,
        q_min_x: f32,
        q_min_y: f32,
        q_max_x: f32,
        q_max_y: f32,
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
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = use_avx2;
        }
        #[cfg(target_arch = "x86_64")]
        if use_avx2 {
            use std::arch::x86_64::*;
            let qminx = _mm256_set1_ps(q_min_x);
            let qmaxx = _mm256_set1_ps(q_max_x);
            let qminy = _mm256_set1_ps(q_min_y);
            let qmaxy = _mm256_set1_ps(q_max_y);
            while idx + 8 <= end {
                #[cfg(feature = "query_stats")]
                {
                    Self::bump_query_entity_ptr(stats);
                    Self::bump_query_entity_ptr(stats);
                    Self::bump_query_entity_ptr(stats);
                    Self::bump_query_entity_ptr(stats);
                    Self::bump_query_entity_ptr(stats);
                    Self::bump_query_entity_ptr(stats);
                    Self::bump_query_entity_ptr(stats);
                    Self::bump_query_entity_ptr(stats);
                }
                let base = idx;
                let min_x = _mm256_loadu_ps(node_entity_min_x_ptr.add(base));
                let min_y = _mm256_loadu_ps(node_entity_min_y_ptr.add(base));
                let max_x = _mm256_loadu_ps(node_entity_max_x_ptr.add(base));
                let max_y = _mm256_loadu_ps(node_entity_max_y_ptr.add(base));
                let x_ok = _mm256_and_ps(
                    _mm256_cmp_ps(max_x, qminx, _CMP_GT_OQ),
                    _mm256_cmp_ps(qmaxx, min_x, _CMP_GT_OQ),
                );
                let y_ok = _mm256_and_ps(
                    _mm256_cmp_ps(max_y, qminy, _CMP_GT_OQ),
                    _mm256_cmp_ps(qmaxy, min_y, _CMP_GT_OQ),
                );
                let mask = _mm256_movemask_ps(_mm256_and_ps(x_ok, y_ok)) as u32;
                if (mask & 0b0000_0001) != 0 {
                    f(*node_entity_values_ptr.add(base));
                }
                if (mask & 0b0000_0010) != 0 {
                    f(*node_entity_values_ptr.add(base + 1));
                }
                if (mask & 0b0000_0100) != 0 {
                    f(*node_entity_values_ptr.add(base + 2));
                }
                if (mask & 0b0000_1000) != 0 {
                    f(*node_entity_values_ptr.add(base + 3));
                }
                if (mask & 0b0001_0000) != 0 {
                    f(*node_entity_values_ptr.add(base + 4));
                }
                if (mask & 0b0010_0000) != 0 {
                    f(*node_entity_values_ptr.add(base + 5));
                }
                if (mask & 0b0100_0000) != 0 {
                    f(*node_entity_values_ptr.add(base + 6));
                }
                if (mask & 0b1000_0000) != 0 {
                    f(*node_entity_values_ptr.add(base + 7));
                }
                idx += 8;
            }
        }


        #[cfg(target_arch = "x86_64")]
        let (qminx, qmaxx, qminy, qmaxy) = {
            use std::arch::x86_64::*;
            (
                _mm_set1_ps(q_min_x),
                _mm_set1_ps(q_max_x),
                _mm_set1_ps(q_min_y),
                _mm_set1_ps(q_max_y),
            )
        };
        while idx + 4 <= end {
            #[cfg(feature = "query_stats")]
            {
                Self::bump_query_entity_ptr(stats);
                Self::bump_query_entity_ptr(stats);
                Self::bump_query_entity_ptr(stats);
                Self::bump_query_entity_ptr(stats);
            }
            let base = idx;
            #[cfg(target_arch = "x86_64")]
            {
                use std::arch::x86_64::*;
                let min_x = _mm_loadu_ps(node_entity_min_x_ptr.add(base));
                let min_y = _mm_loadu_ps(node_entity_min_y_ptr.add(base));
                let max_x = _mm_loadu_ps(node_entity_max_x_ptr.add(base));
                let max_y = _mm_loadu_ps(node_entity_max_y_ptr.add(base));
                let x_ok = _mm_and_ps(_mm_cmpgt_ps(max_x, qminx), _mm_cmpgt_ps(qmaxx, min_x));
                let y_ok = _mm_and_ps(_mm_cmpgt_ps(max_y, qminy), _mm_cmpgt_ps(qmaxy, min_y));
                let mask = _mm_movemask_ps(_mm_and_ps(x_ok, y_ok)) as u32;
                if (mask & 0b0001) != 0 {
                    f(*node_entity_values_ptr.add(base));
                }
                if (mask & 0b0010) != 0 {
                    f(*node_entity_values_ptr.add(base + 1));
                }
                if (mask & 0b0100) != 0 {
                    f(*node_entity_values_ptr.add(base + 2));
                }
                if (mask & 0b1000) != 0 {
                    f(*node_entity_values_ptr.add(base + 3));
                }
            }
            idx += 4;
        }
        while idx < end {
            #[cfg(feature = "query_stats")]
            Self::bump_query_entity_ptr(stats);
            let min_x = *node_entity_min_x_ptr.add(idx);
            let max_x = *node_entity_max_x_ptr.add(idx);
            if max_x <= q_min_x || q_max_x <= min_x {
                idx += 1;
                continue;
            }
            let min_y = *node_entity_min_y_ptr.add(idx);
            let max_y = *node_entity_max_y_ptr.add(idx);
            if max_y <= q_min_y || q_max_y <= min_y {
                idx += 1;
                continue;
            }
            let value = *node_entity_values_ptr.add(idx);
            f(value);
            idx += 1;
        }
    }
}
