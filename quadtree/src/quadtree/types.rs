use common::shapes::{Circle, Rectangle, ShapeEnum};
use fxhash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use std::cell::RefCell;

const FLAG_LEFT: u8 = 0b0001;
const FLAG_BOTTOM: u8 = 0b0010;
const FLAG_RIGHT: u8 = 0b0100;
const FLAG_TOP: u8 = 0b1000;
const NODE_ENTITY_DEDUPE_MASK: u32 = 0x8000_0000;
const NODE_ENTITY_INDEX_MASK: u32 = 0x7FFF_FFFF;
const SHAPE_RECT: u8 = 0;
const SHAPE_CIRCLE: u8 = 1;

#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
struct RectExtent {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl RectExtent {
    #[inline(always)]
    fn from_rect(rect: &Rectangle) -> Self {
        let half_w = rect.width.abs() * 0.5;
        let half_h = rect.height.abs() * 0.5;
        Self {
            min_x: rect.x - half_w,
            min_y: rect.y - half_h,
            max_x: rect.x + half_w,
            max_y: rect.y + half_h,
        }
    }

    #[inline(always)]
    fn from_min_max(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        let (min_x, max_x) = if min_x <= max_x {
            (min_x, max_x)
        } else {
            (max_x, min_x)
        };
        let (min_y, max_y) = if min_y <= max_y {
            (min_y, max_y)
        } else {
            (max_y, min_y)
        };
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

}

#[derive(Clone, Copy, Debug)]
struct HalfExtent {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl HalfExtent {
    #[inline(always)]
    fn from_rect_extent(extent: RectExtent) -> Self {
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
    fn to_rect_extent(self) -> RectExtent {
        RectExtent {
            min_x: self.x - self.w,
            min_y: self.y - self.h,
            max_x: self.x + self.w,
            max_y: self.y + self.h,
        }
    }
}

#[inline(always)]
fn loose_half_extent(half: HalfExtent, looseness: f32) -> HalfExtent {
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
fn loose_extent_from_half(half: HalfExtent, looseness: f32) -> RectExtent {
    loose_half_extent(half, looseness).to_rect_extent()
}

#[inline(always)]
fn extent_fits_in_loose_half(half: HalfExtent, extent: RectExtent, looseness: f32) -> bool {
    let loose = loose_half_extent(half, looseness);
    extent.min_x >= loose.x - loose.w
        && extent.max_x <= loose.x + loose.w
        && extent.min_y >= loose.y - loose.h
        && extent.max_y <= loose.y + loose.h
}

#[inline(always)]
fn child_targets_for_extent(
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

type NodeStack = SmallVec<[(u32, HalfExtent); 64]>;

#[inline(always)]
fn point_to_extent_distance_sq(x: f32, y: f32, extent: RectExtent) -> f32 {
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
fn circle_circle_raw(x1: f32, y1: f32, r1: f32, x2: f32, y2: f32, r2: f32) -> bool {
    let dx = x1 - x2;
    let dy = y1 - y2;
    let r = r1 + r2;
    dx * dx + dy * dy < r * r
}

#[inline(always)]
fn circle_rect_raw(
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
fn circle_extent_raw(cx: f32, cy: f32, radius: f32, radius_sq: f32, extent: RectExtent) -> bool {
    let rect_x = (extent.min_x + extent.max_x) * 0.5;
    let rect_y = (extent.min_y + extent.max_y) * 0.5;
    let half_w = (extent.max_x - extent.min_x) * 0.5;
    let half_h = (extent.max_y - extent.min_y) * 0.5;
    circle_rect_raw(cx, cy, radius, radius_sq, rect_x, rect_y, half_w, half_h)
}

#[derive(Clone, Copy)]
struct CircleData {
    x: f32,
    y: f32,
    radius: f32,
    radius_sq: f32,
}

impl CircleData {
    fn new(x: f32, y: f32, radius: f32) -> Self {
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
struct Entity {
    next_free: u32,
    in_nodes_minus_one: u32,
    update_tick: u8,
    reinsertion_tick: u8,
    status_changed: u8,
    alive: u8,
    shape_kind: u8,
    _padding: [u8; 3],
}

impl Entity {
    fn sentinel() -> Self {
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
enum QueryKind {
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
struct Query {
    extent: RectExtent,
    kind: QueryKind,
}

impl Query {
    fn from_shape(shape: &ShapeEnum) -> Self {
        match shape {
            ShapeEnum::Rectangle(rect) => Self {
                extent: RectExtent::from_rect(rect),
                kind: QueryKind::Rect {
                    x: rect.x,
                    y: rect.y,
                    half_w: rect.width * 0.5,
                    half_h: rect.height * 0.5,
                },
            },
            ShapeEnum::Circle(circle) => {
                let radius = circle.radius;
                Self {
                    extent: RectExtent::from_min_max(
                        circle.x - radius,
                        circle.y - radius,
                        circle.x + radius,
                        circle.y + radius,
                    ),
                    kind: QueryKind::Circle {
                        x: circle.x,
                        y: circle.y,
                        radius,
                        radius_sq: radius * radius,
                    },
                }
            }
        }
    }

    #[inline(always)]
    fn from_rect_extent(extent: RectExtent) -> Self {
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
    fn from_circle_raw(x: f32, y: f32, radius: f32) -> Self {
        Self {
            extent: RectExtent::from_min_max(x - radius, y - radius, x + radius, y + radius),
            kind: QueryKind::Circle {
                x,
                y,
                radius,
                radius_sq: radius * radius,
            },
        }
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
struct NodeEntity(u32);

impl NodeEntity {
    #[inline(always)]
    fn new(index: u32) -> Self {
        NodeEntity(index & NODE_ENTITY_INDEX_MASK)
    }

    #[inline(always)]
    fn index(self) -> u32 {
        self.0 & NODE_ENTITY_INDEX_MASK
    }

    #[inline(always)]
    fn raw(self) -> u32 {
        self.0
    }

    #[inline(always)]
    fn from_raw(raw: u32) -> Self {
        NodeEntity(raw)
    }

    #[inline(always)]
    fn has_dedupe(self) -> bool {
        (self.0 & NODE_ENTITY_DEDUPE_MASK) != 0
    }

    #[inline(always)]
    fn set_index(&mut self, index: u32) {
        self.0 = (self.0 & NODE_ENTITY_DEDUPE_MASK) | (index & NODE_ENTITY_INDEX_MASK);
    }

    #[inline(always)]
    fn set_dedupe(&mut self, dedupe: bool) {
        if dedupe {
            self.0 |= NODE_ENTITY_DEDUPE_MASK;
        } else {
            self.0 &= NODE_ENTITY_INDEX_MASK;
        }
    }
}


#[repr(C)]
#[derive(Clone, Copy, Default)]
struct NodeEntityPacked {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    value: u32,
    entity_raw: u32,
}

impl NodeEntityPacked {
    #[inline(always)]
    fn from_parts(extent: RectExtent, value: u32, entity: NodeEntity) -> Self {
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
    fn set_extent(&mut self, extent: RectExtent) {
        self.min_x = extent.min_x;
        self.min_y = extent.min_y;
        self.max_x = extent.max_x;
        self.max_y = extent.max_y;
    }

    #[inline(always)]
    fn set_entity(&mut self, entity: NodeEntity) {
        self.entity_raw = entity.raw();
    }

    #[inline(always)]
    fn entity(self) -> NodeEntity {
        NodeEntity::from_raw(self.entity_raw)
    }

    #[inline(always)]
    fn value(self) -> u32 {
        self.value
    }
}

struct EntityReorder {
    old_entities: *const Entity,
    new_entities: *mut Entity,
    old_extents: *const RectExtent,
    new_extents: *mut RectExtent,
    old_values: *const u32,
    new_values: *mut u32,
    old_types: *const u32,
    new_types: *mut u32,
    old_circle_data: *const CircleData,
    new_circle_data: *mut CircleData,
    entity_map: *mut u32,
    entity_map_len: usize,
    new_len: usize,
    circle_count: u32,
    alive_count: u32,
    all_rectangles: bool,
    all_circles: bool,
    has_entity_types: bool,
}


trait EntityMapper {
    fn map_entity(&mut self, old_idx: u32, in_nodes_minus_one: u32) -> u32;
    fn update_in_nodes_if_mapped(&mut self, old_idx: u32, in_nodes_minus_one: u32);
}

struct IdentityMapper;

impl EntityMapper for IdentityMapper {
    #[inline(always)]
    fn map_entity(&mut self, old_idx: u32, _in_nodes_minus_one: u32) -> u32 {
        old_idx
    }

    #[inline(always)]
    fn update_in_nodes_if_mapped(&mut self, _old_idx: u32, _in_nodes_minus_one: u32) {}
}
