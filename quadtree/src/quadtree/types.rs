use super::QuadTreeInner;
use crate::error::{QuadtreeError, QuadtreeResult};
use common::shapes::{Rectangle, ShapeEnum};
use smallvec::SmallVec;

pub(crate) const FLAG_LEFT: u8 = 0b0001;
pub(crate) const FLAG_BOTTOM: u8 = 0b0010;
pub(crate) const FLAG_RIGHT: u8 = 0b0100;
pub(crate) const FLAG_TOP: u8 = 0b1000;
pub(crate) const NODE_ENTITY_DEDUPE_MASK: u32 = 0x8000_0000;
pub(crate) const NODE_ENTITY_INDEX_MASK: u32 = 0x7FFF_FFFF;
pub(crate) const SHAPE_RECT: u8 = 0;
pub(crate) const SHAPE_CIRCLE: u8 = 1;

#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub(crate) struct RectExtent {
    pub(crate) min_x: f32,
    pub(crate) min_y: f32,
    pub(crate) max_x: f32,
    pub(crate) max_y: f32,
}

impl RectExtent {
    #[inline(always)]
    pub(crate) fn from_rect(rect: &Rectangle) -> QuadtreeResult<Self> {
        validate_rect_dims(rect.width, rect.height)?;
        let half_w = rect.width * 0.5;
        let half_h = rect.height * 0.5;
        Ok(Self {
            min_x: rect.x - half_w,
            min_y: rect.y - half_h,
            max_x: rect.x + half_w,
            max_y: rect.y + half_h,
        })
    }

    #[inline(always)]
    pub(crate) fn from_min_max(
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
    ) -> QuadtreeResult<Self> {
        validate_rect_extent_bounds(min_x, min_y, max_x, max_y)?;
        Ok(Self {
            min_x,
            min_y,
            max_x,
            max_y,
        })
    }

    #[inline(always)]
    pub(crate) fn from_min_max_unchecked(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct HalfExtent {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
}

impl HalfExtent {
    #[inline(always)]
    pub(crate) fn from_rect_extent(extent: RectExtent) -> Self {
        let half_w = (extent.max_x - extent.min_x) * 0.5;
        let half_h = (extent.max_y - extent.min_y) * 0.5;
        Self {
            x: extent.min_x + half_w,
            y: extent.min_y + half_h,
            w: half_w,
            h: half_h,
        }
    }

    #[inline(always)]
    pub(crate) fn to_rect_extent(self) -> RectExtent {
        RectExtent {
            min_x: self.x - self.w,
            min_y: self.y - self.h,
            max_x: self.x + self.w,
            max_y: self.y + self.h,
        }
    }
}

#[inline(always)]
pub(crate) fn loose_half_extent(half: HalfExtent, looseness: f32) -> HalfExtent {
    if looseness <= 1.0 {
        half
    } else {
        HalfExtent {
            x: half.x,
            y: half.y,
            w: half.w * looseness,
            h: half.h * looseness,
        }
    }
}

#[inline(always)]
pub(crate) fn loose_extent_from_half(half: HalfExtent, looseness: f32) -> RectExtent {
    loose_half_extent(half, looseness).to_rect_extent()
}

#[inline(always)]
pub(crate) fn extent_fits_in_loose_half(
    half: HalfExtent,
    extent: RectExtent,
    looseness: f32,
) -> bool {
    let loose = loose_half_extent(half, looseness);
    extent.min_x >= loose.x - loose.w
        && extent.max_x <= loose.x + loose.w
        && extent.min_y >= loose.y - loose.h
        && extent.max_y <= loose.y + loose.h
}

#[inline(always)]
pub(crate) fn child_targets_for_extent(
    half: HalfExtent,
    extent: RectExtent,
    looseness: f32,
    targets: &mut [usize; 4],
) -> usize {
    if looseness > 1.0 {
        let mut single = None;
        for i in 0..4 {
            let child_half = QuadTreeInner::child_half_extent(half, i);
            if extent_fits_in_loose_half(child_half, extent, looseness) {
                if single.is_some() {
                    single = None;
                    break;
                }
                single = Some(i);
            }
        }
        if let Some(index) = single {
            targets[0] = index;
            return 1;
        }
    }

    let mut targets_len = 0usize;
    if extent.min_x <= half.x {
        if extent.min_y <= half.y {
            targets[targets_len] = 0;
            targets_len += 1;
        }
        if extent.max_y >= half.y {
            targets[targets_len] = 1;
            targets_len += 1;
        }
    }
    if extent.max_x >= half.x {
        if extent.min_y <= half.y {
            targets[targets_len] = 2;
            targets_len += 1;
        }
        if extent.max_y >= half.y {
            targets[targets_len] = 3;
            targets_len += 1;
        }
    }

    targets_len
}

pub(crate) type NodeStack = SmallVec<[(u32, HalfExtent); 64]>;

#[inline(always)]
pub(crate) fn point_to_extent_distance_sq(x: f32, y: f32, extent: RectExtent) -> f32 {
    let dx = if x < extent.min_x {
        extent.min_x - x
    } else if x > extent.max_x {
        x - extent.max_x
    } else {
        0.0
    };

    let dy = if y < extent.min_y {
        extent.min_y - y
    } else if y > extent.max_y {
        y - extent.max_y
    } else {
        0.0
    };

    dx * dx + dy * dy
}

#[inline(always)]
pub(crate) fn circle_circle_raw(x1: f32, y1: f32, r1: f32, x2: f32, y2: f32, r2: f32) -> bool {
    let dx = x1 - x2;
    let dy = y1 - y2;
    let r = r1 + r2;
    dx * dx + dy * dy < r * r
}

#[inline(always)]
pub(crate) fn circle_rect_raw(
    cx: f32,
    cy: f32,
    radius: f32,
    radius_sq: f32,
    rect_x: f32,
    rect_y: f32,
    half_w: f32,
    half_h: f32,
) -> bool {
    let dx = (cx - rect_x).abs();
    let dy = (cy - rect_y).abs();
    if dx >= half_w + radius || dy >= half_h + radius {
        return false;
    }
    if dx < half_w || dy < half_h {
        return true;
    }
    let corner_dx = dx - half_w;
    let corner_dy = dy - half_h;
    corner_dx * corner_dx + corner_dy * corner_dy < radius_sq
}

#[inline(always)]
pub(crate) fn circle_extent_raw(
    cx: f32,
    cy: f32,
    radius: f32,
    radius_sq: f32,
    extent: RectExtent,
) -> bool {
    let rect_x = (extent.min_x + extent.max_x) * 0.5;
    let rect_y = (extent.min_y + extent.max_y) * 0.5;
    let half_w = (extent.max_x - extent.min_x) * 0.5;
    let half_h = (extent.max_y - extent.min_y) * 0.5;
    circle_rect_raw(cx, cy, radius, radius_sq, rect_x, rect_y, half_w, half_h)
}

#[derive(Clone, Copy)]
pub(crate) struct CircleData {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) radius: f32,
    pub(crate) radius_sq: f32,
}

impl CircleData {
    pub(crate) fn new(x: f32, y: f32, radius: f32) -> Self {
        Self {
            x,
            y,
            radius,
            radius_sq: radius * radius,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct Entity {
    pub(crate) next_free: u32,
    pub(crate) in_nodes_minus_one: u32,
    pub(crate) update_tick: u8,
    pub(crate) reinsertion_tick: u8,
    pub(crate) status_changed: u8,
    pub(crate) alive: u8,
    pub(crate) shape_kind: u8,
    pub(crate) _padding: [u8; 3],
}

impl Entity {
    pub(crate) fn sentinel() -> Self {
        Self {
            next_free: 0,
            in_nodes_minus_one: 0,
            update_tick: 0,
            reinsertion_tick: 0,
            status_changed: 0,
            alive: 0,
            shape_kind: SHAPE_RECT,
            _padding: [0; 3],
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum QueryKind {
    Rect {
        x: f32,
        y: f32,
        half_w: f32,
        half_h: f32,
    },
    Circle {
        x: f32,
        y: f32,
        radius: f32,
        radius_sq: f32,
    },
}

#[derive(Clone, Copy)]
pub(crate) struct Query {
    pub(crate) extent: RectExtent,
    pub(crate) kind: QueryKind,
}

impl Query {
    pub(crate) fn from_shape(shape: &ShapeEnum) -> QuadtreeResult<Self> {
        match shape {
            ShapeEnum::Rectangle(rect) => Ok(Self {
                extent: RectExtent::from_rect(rect)?,
                kind: QueryKind::Rect {
                    x: rect.x,
                    y: rect.y,
                    half_w: rect.width * 0.5,
                    half_h: rect.height * 0.5,
                },
            }),
            ShapeEnum::Circle(circle) => {
                let radius = circle.radius;
                validate_circle_radius(radius)?;
                Ok(Self {
                    extent: RectExtent::from_min_max(
                        circle.x - radius,
                        circle.y - radius,
                        circle.x + radius,
                        circle.y + radius,
                    )?,
                    kind: QueryKind::Circle {
                        x: circle.x,
                        y: circle.y,
                        radius,
                        radius_sq: radius * radius,
                    },
                })
            }
        }
    }

    #[inline(always)]
    pub(crate) fn from_rect_extent(extent: RectExtent) -> Self {
        Self {
            extent,
            kind: QueryKind::Rect {
                x: (extent.min_x + extent.max_x) * 0.5,
                y: (extent.min_y + extent.max_y) * 0.5,
                half_w: (extent.max_x - extent.min_x) * 0.5,
                half_h: (extent.max_y - extent.min_y) * 0.5,
            },
        }
    }

    #[inline(always)]
    pub(crate) fn from_circle_raw(x: f32, y: f32, radius: f32) -> QuadtreeResult<Self> {
        validate_circle_radius(radius)?;
        Ok(Self {
            extent: RectExtent::from_min_max(x - radius, y - radius, x + radius, y + radius)?,
            kind: QueryKind::Circle {
                x,
                y,
                radius,
                radius_sq: radius * radius,
            },
        })
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub(crate) struct NodeEntity(u32);

impl NodeEntity {
    #[inline(always)]
    pub(crate) fn new(index: u32) -> Self {
        debug_assert!(
            index <= NODE_ENTITY_INDEX_MASK,
            "entity index {} exceeds max {}",
            index,
            NODE_ENTITY_INDEX_MASK
        );
        NodeEntity(index & NODE_ENTITY_INDEX_MASK)
    }

    #[inline(always)]
    pub(crate) fn index(self) -> u32 {
        self.0 & NODE_ENTITY_INDEX_MASK
    }

    #[inline(always)]
    pub(crate) fn raw(self) -> u32 {
        self.0
    }

    #[inline(always)]
    pub(crate) fn from_raw(raw: u32) -> Self {
        NodeEntity(raw)
    }

    #[inline(always)]
    pub(crate) fn has_dedupe(self) -> bool {
        (self.0 & NODE_ENTITY_DEDUPE_MASK) != 0
    }

    #[inline(always)]
    pub(crate) fn set_index(&mut self, index: u32) {
        debug_assert!(
            index <= NODE_ENTITY_INDEX_MASK,
            "entity index {} exceeds max {}",
            index,
            NODE_ENTITY_INDEX_MASK
        );
        self.0 = (self.0 & NODE_ENTITY_DEDUPE_MASK) | (index & NODE_ENTITY_INDEX_MASK);
    }

    #[inline(always)]
    pub(crate) fn set_dedupe(&mut self, dedupe: bool) {
        if dedupe {
            self.0 |= NODE_ENTITY_DEDUPE_MASK;
        } else {
            self.0 &= NODE_ENTITY_INDEX_MASK;
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(crate) struct NodeEntityPacked {
    pub(crate) min_x: f32,
    pub(crate) min_y: f32,
    pub(crate) max_x: f32,
    pub(crate) max_y: f32,
    pub(crate) value: u32,
    pub(crate) entity_raw: u32,
}

impl NodeEntityPacked {
    #[inline(always)]
    pub(crate) fn from_parts(extent: RectExtent, value: u32, entity: NodeEntity) -> Self {
        Self {
            min_x: extent.min_x,
            min_y: extent.min_y,
            max_x: extent.max_x,
            max_y: extent.max_y,
            value,
            entity_raw: entity.raw(),
        }
    }

    #[inline(always)]
    pub(crate) fn set_extent(&mut self, extent: RectExtent) {
        self.min_x = extent.min_x;
        self.min_y = extent.min_y;
        self.max_x = extent.max_x;
        self.max_y = extent.max_y;
    }

    #[inline(always)]
    pub(crate) fn set_entity(&mut self, entity: NodeEntity) {
        self.entity_raw = entity.raw();
    }

    #[inline(always)]
    pub(crate) fn entity(self) -> NodeEntity {
        NodeEntity::from_raw(self.entity_raw)
    }

    #[inline(always)]
    pub(crate) fn value(self) -> u32 {
        self.value
    }
}

pub(crate) struct EntityReorder {
    pub(crate) old_entities: *const Entity,
    pub(crate) new_entities: *mut Entity,
    pub(crate) old_extents: *const RectExtent,
    pub(crate) new_extents: *mut RectExtent,
    pub(crate) old_values: *const u32,
    pub(crate) new_values: *mut u32,
    pub(crate) old_types: *const u32,
    pub(crate) new_types: *mut u32,
    pub(crate) old_circle_data: *const CircleData,
    pub(crate) new_circle_data: *mut CircleData,
    pub(crate) entity_map: *mut u32,
    pub(crate) entity_map_len: usize,
    pub(crate) new_len: usize,
    pub(crate) circle_count: u32,
    pub(crate) alive_count: u32,
    pub(crate) all_rectangles: bool,
    pub(crate) all_circles: bool,
    pub(crate) has_entity_types: bool,
}

pub(crate) trait EntityMapper {
    fn map_entity(&mut self, old_idx: u32, in_nodes_minus_one: u32) -> u32;
    fn update_in_nodes_if_mapped(&mut self, old_idx: u32, in_nodes_minus_one: u32);
}

pub(crate) struct IdentityMapper;

impl EntityMapper for IdentityMapper {
    #[inline(always)]
    fn map_entity(&mut self, old_idx: u32, _in_nodes_minus_one: u32) -> u32 {
        old_idx
    }

    #[inline(always)]
    fn update_in_nodes_if_mapped(&mut self, _old_idx: u32, _in_nodes_minus_one: u32) {}
}

pub(crate) fn validate_rect_dims(width: f32, height: f32) -> QuadtreeResult<()> {
    if !(width.is_finite() && height.is_finite()) || width < 0.0 || height < 0.0 {
        return Err(QuadtreeError::InvalidRectangleDims { width, height });
    }
    Ok(())
}

pub(crate) fn validate_circle_radius(radius: f32) -> QuadtreeResult<()> {
    if !(radius.is_finite() && radius >= 0.0) {
        return Err(QuadtreeError::InvalidCircleRadius { radius });
    }
    Ok(())
}

pub(crate) fn validate_rect_extent_bounds(
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
) -> QuadtreeResult<()> {
    if !(min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite())
        || min_x > max_x
        || min_y > max_y
    {
        return Err(QuadtreeError::InvalidRectExtent {
            min_x,
            min_y,
            max_x,
            max_y,
        });
    }
    Ok(())
}
