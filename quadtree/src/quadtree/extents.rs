#[derive(Default)]
pub(crate) struct ExtentAos {
    pub(crate) extents: Vec<RectExtent>,
}

impl ExtentAos {
    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        self.extents.len()
    }

    #[inline(always)]
    pub(crate) fn clear(&mut self) {
        self.extents.clear();
    }

    #[inline(always)]
    pub(crate) fn reserve(&mut self, additional: usize) {
        self.extents.reserve(additional);
    }

    #[inline(always)]
    pub(crate) fn push(&mut self, extent: RectExtent) {
        self.extents.push(extent);
    }

    #[inline(always)]
    pub(crate) fn set(&mut self, idx: usize, extent: RectExtent) {
        self.extents[idx] = extent;
    }

    #[inline(always)]
    pub(crate) fn extent(&self, idx: usize) -> RectExtent {
        self.extents[idx]
    }

    #[inline(always)]
    pub(crate) fn ptr(&self) -> *const RectExtent {
        self.extents.as_ptr()
    }
}

#[derive(Default)]
pub(crate) struct NodeEntityExtentsSoa {
    min_x: Vec<f32>,
    min_y: Vec<f32>,
    max_x: Vec<f32>,
    max_y: Vec<f32>,
}

impl NodeEntityExtentsSoa {
    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        self.min_x.len()
    }

    #[inline(always)]
    pub(crate) fn clear(&mut self) {
        self.min_x.clear();
        self.min_y.clear();
        self.max_x.clear();
        self.max_y.clear();
    }

    #[inline(always)]
    pub(crate) fn reserve(&mut self, additional: usize) {
        self.min_x.reserve(additional);
        self.min_y.reserve(additional);
        self.max_x.reserve(additional);
        self.max_y.reserve(additional);
    }

    #[inline(always)]
    pub(crate) fn push(&mut self, extent: RectExtent) {
        self.min_x.push(extent.min_x);
        self.min_y.push(extent.min_y);
        self.max_x.push(extent.max_x);
        self.max_y.push(extent.max_y);
    }

    #[inline(always)]
    pub(crate) fn resize(&mut self, new_len: usize) {
        if self.min_x.len() < new_len {
            self.min_x.resize(new_len, 0.0);
            self.min_y.resize(new_len, 0.0);
            self.max_x.resize(new_len, 0.0);
            self.max_y.resize(new_len, 0.0);
        }
    }

    #[inline(always)]
    pub(crate) fn set(&mut self, idx: usize, extent: RectExtent) {
        self.min_x[idx] = extent.min_x;
        self.min_y[idx] = extent.min_y;
        self.max_x[idx] = extent.max_x;
        self.max_y[idx] = extent.max_y;
    }

    #[inline(always)]
    pub(crate) fn extent(&self, idx: usize) -> RectExtent {
        RectExtent {
            min_x: self.min_x[idx],
            min_y: self.min_y[idx],
            max_x: self.max_x[idx],
            max_y: self.max_y[idx],
        }
    }

    #[inline(always)]
    pub(crate) fn min_x_ptr(&self) -> *const f32 {
        self.min_x.as_ptr()
    }

    #[inline(always)]
    pub(crate) fn min_x_mut_ptr(&mut self) -> *mut f32 {
        self.min_x.as_mut_ptr()
    }

    #[inline(always)]
    pub(crate) fn min_y_mut_ptr(&mut self) -> *mut f32 {
        self.min_y.as_mut_ptr()
    }

    #[inline(always)]
    pub(crate) fn max_x_mut_ptr(&mut self) -> *mut f32 {
        self.max_x.as_mut_ptr()
    }

    #[inline(always)]
    pub(crate) fn max_y_mut_ptr(&mut self) -> *mut f32 {
        self.max_y.as_mut_ptr()
    }

    #[inline(always)]
    pub(crate) fn min_y_ptr(&self) -> *const f32 {
        self.min_y.as_ptr()
    }

    #[inline(always)]
    pub(crate) fn max_x_ptr(&self) -> *const f32 {
        self.max_x.as_ptr()
    }

    #[inline(always)]
    pub(crate) fn max_y_ptr(&self) -> *const f32 {
        self.max_y.as_ptr()
    }
}
use super::RectExtent;
