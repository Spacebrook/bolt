# 🌳 Bolt
High-performance Physics implementation in Rust, built for evades.io. This library provides a Python binding for easy integration and use in Python projects. The quadtree data structure is useful for spatial partitioning and efficient collision detection.

With Quadtree, you can effortlessly manage spatial data and efficiently perform collision detection. Happy coding! 🚀

## 🦀 Usage in Rust
Here's an example of how to use the quadtree library in Rust:

```rust
use bolt_quadtree::shapes::{Circle, Rectangle, ShapeEnum};
use quadtree::{QuadtreeResult};
use quadtree::quadtree::{EntityTypeUpdate, QuadTree};

fn main() -> QuadtreeResult<()> {
    // 🖼️ Create a bounding rectangle for the quadtree
    let bounding_box = Rectangle {
        x: 0.0,
        y: 0.0,
        width: 10.0,
        height: 10.0,
    };

    // 🌳 Create a QuadTree with the given bounding box
    let mut quadtree = QuadTree::new(bounding_box)?;

    // 🟩 Create instances of Rectangle and Circle
    let rect1 = Rectangle {
        x: 2.0,
        y: 3.0,
        width: 4.0,
        height: 5.0,
    };
    let circle1 = Circle::new(6.0, 7.0, 2.0);

    // ➕ Insert shapes into the quadtree with associated values
    quadtree.insert(1, ShapeEnum::Rectangle(rect1), None)?;
    quadtree.insert(2, ShapeEnum::Circle(circle1), None)?;

    // 🔍 Check for collisions with a given shape
    let mut collisions_with_rect = Vec::new();
    quadtree.collisions(ShapeEnum::Rectangle(rect1), &mut collisions_with_rect)?;
    println!("Collisions with rect1: {:?}", collisions_with_rect);

    // 🔁 Relocate an existing shape in the quadtree (preserve entity type)
    let rect2 = Rectangle {
        x: 5.0,
        y: 5.0,
        width: 1.0,
        height: 1.0,
    };
    quadtree.relocate(1, ShapeEnum::Rectangle(rect2), EntityTypeUpdate::Preserve)?;

    // 🔍 Check for collisions with a given shape after relocation
    let mut collisions_with_rect2 = Vec::new();
    quadtree.collisions(ShapeEnum::Rectangle(rect2), &mut collisions_with_rect2)?;
    println!("Collisions with rect2: {:?}", collisions_with_rect2);

    // ❌ Delete a shape from the quadtree
    quadtree.delete(2);

    // 📋 Get all shapes in the quadtree
    let mut all_shapes = Vec::new();
    quadtree.all_shapes(&mut all_shapes);
    println!("All shapes: {:?}", all_shapes);

    // 📐 Get all node bounding boxes in the quadtree
    let mut all_bounding_boxes = Vec::new();
    quadtree.all_node_bounding_boxes(&mut all_bounding_boxes);
    println!("All bounding boxes: {:?}", all_bounding_boxes);

    Ok(())
}
```

## 📚 Rust API
Public entry points and signatures:

```rust
// Configuration and inputs.
pub struct Config { pub pool_size: usize, pub node_capacity: usize, pub max_depth: usize, pub min_size: f32, pub looseness: f32, pub large_entity_threshold_factor: f32 }
pub enum EntityTypeUpdate { Preserve, Clear, Set(u32) }
pub struct RelocationRequest { pub value: u32, pub shape: ShapeEnum, pub entity_type: EntityTypeUpdate }
pub struct QueryStats { pub query_calls: u64, pub node_visits: u64, pub entity_visits: u64 } // feature: query_stats

// QuadTree construction and diagnostics.
pub fn QuadTree::new(bounding_box: Rectangle) -> QuadtreeResult<QuadTree> // create with default config
pub fn QuadTree::new_with_config(bounding_box: Rectangle, config: Config) -> QuadtreeResult<QuadTree> // create with custom config
pub fn QuadTree::storage_counts(&self) -> (usize, usize, usize) // (nodes, node_entities, entities)

// Insert and delete.
pub fn QuadTree::insert(&mut self, value: u32, shape: ShapeEnum, entity_type: Option<u32>) -> QuadtreeResult<()> // insert shape/value
pub fn QuadTree::insert_rect_extent(&mut self, value: u32, min_x: f32, min_y: f32, max_x: f32, max_y: f32, entity_type: Option<u32>) -> QuadtreeResult<()> // insert rectangle by bounds
pub fn QuadTree::insert_circle_raw(&mut self, value: u32, x: f32, y: f32, radius: f32, entity_type: Option<u32>) -> QuadtreeResult<()> // insert circle by raw params
pub fn QuadTree::delete(&mut self, value: u32) // remove by value

// Relocate and maintenance.
pub fn QuadTree::relocate_batch(&mut self, relocation_requests: Vec<RelocationRequest>) -> QuadtreeResult<()> // batch relocate
pub fn QuadTree::relocate(&mut self, value: u32, shape: ShapeEnum, entity_type: EntityTypeUpdate) -> QuadtreeResult<()> // relocate by shape
pub fn QuadTree::relocate_rect_extent(&mut self, value: u32, min_x: f32, min_y: f32, max_x: f32, max_y: f32, entity_type: EntityTypeUpdate) -> QuadtreeResult<()> // relocate rectangle by bounds
pub fn QuadTree::relocate_circle_raw(&mut self, value: u32, x: f32, y: f32, radius: f32, entity_type: EntityTypeUpdate) -> QuadtreeResult<()> // relocate circle by raw params
pub fn QuadTree::update(&self) // apply pending updates

// Entity type updates:
// EntityTypeUpdate::Preserve keeps the existing type, Clear removes it, and Set(...) overwrites it.

// Query results into a Vec.
pub fn QuadTree::collisions(&self, shape: ShapeEnum, collisions: &mut Vec<u32>) -> QuadtreeResult<()> // append hits (touching edges are not collisions)
pub fn QuadTree::collisions_rect_extent(&self, min_x: f32, min_y: f32, max_x: f32, max_y: f32, collisions: &mut Vec<u32>) -> QuadtreeResult<()> // append hits in bounds
pub fn QuadTree::collisions_circle_raw(&self, x: f32, y: f32, radius: f32, collisions: &mut Vec<u32>) -> QuadtreeResult<()> // append hits in circle
pub fn QuadTree::collisions_batch(&self, shapes: Vec<ShapeEnum>) -> QuadtreeResult<Vec<Vec<u32>>> // one vec per shape
pub fn QuadTree::collisions_batch_filter(&self, shapes: Vec<ShapeEnum>, filter_entity_types: Option<Vec<u32>>) -> QuadtreeResult<Vec<Vec<u32>>> // typed batch
pub fn QuadTree::collisions_filter(&self, shape: ShapeEnum, filter_entity_types: Option<Vec<u32>>, collisions: &mut Vec<u32>) -> QuadtreeResult<()> // append filtered hits

// Query callbacks.
pub fn QuadTree::collisions_with<F>(&self, shape: ShapeEnum, f: F) -> QuadtreeResult<()> where F: FnMut(u32) // call for each hit
pub fn QuadTree::collisions_rect_extent_with<F>(&self, min_x: f32, min_y: f32, max_x: f32, max_y: f32, f: F) -> QuadtreeResult<()> where F: FnMut(u32) // call for each hit in bounds
pub fn QuadTree::collisions_circle_raw_with<F>(&self, x: f32, y: f32, radius: f32, f: F) -> QuadtreeResult<()> where F: FnMut(u32) // call for each hit in circle
pub fn QuadTree::collisions_with_filter<F>(&self, shape: ShapeEnum, filter_entity_types: Option<Vec<u32>>, f: F) -> QuadtreeResult<()> where F: FnMut(u32) // filtered callback

// Introspection and stats.
pub fn QuadTree::for_each_collision_pair<F>(&self, f: F) where F: FnMut(u32, u32) // all colliding pairs
pub fn QuadTree::all_node_bounding_boxes(&self, bounding_boxes: &mut Vec<Rectangle>) // dump node bounds
pub fn QuadTree::all_shapes(&self, shapes: &mut Vec<ShapeEnum>) // dump stored shapes
pub fn QuadTree::take_query_stats(&self) -> QueryStats // reset and return stats
pub fn QuadTree::entity_node_stats(&self) -> (f64, u32) // feature: query_stats

// collision_detection helpers.
pub fn rectangle_contains_rectangle(outer: &Rectangle, inner: &Rectangle) -> QuadtreeResult<bool> // containment test
pub fn rectangle_rectangle(a: &Rectangle, b: &Rectangle) -> QuadtreeResult<bool> // rect overlap (touching edges are not collisions)
pub fn circle_circle(a: &Circle, b: &Circle) -> QuadtreeResult<bool> // circle overlap (touching edges are not collisions)
pub fn circle_rectangle(circle: &Circle, rectangle: &Rectangle) -> QuadtreeResult<bool> // circle-rect overlap (touching edges are not collisions)
pub fn shape_shape(a: &ShapeEnum, b: &ShapeEnum) -> QuadtreeResult<bool> // generic overlap (touching edges are not collisions)
```

## 🐍 Usage in Python

### 🛠️ Building the Python Extension
To build the Python extension module, navigate to the python directory and run the following command:

```sh
maturin build --release
```

This will build the Python extension in release mode, and the wheel package will be created in the target/wheels directory.

### 📦 Installing the Package
The generated wheel package can be installed using pip. From the project's root directory, run the following command:

```sh
pip install target/wheels/pyquadtree-*.whl
```

This will install the `pyquadtree` package, and you can start using it in your Python projects.

### 📝 Python Example
Here's an example of how to use the pyquadtree library in Python:

```python
import pyquadtree

# 🟩 Create instances of Rectangle and Circle
rect1 = pyquadtree.Rectangle(2.0, 3.0, 4.0, 5.0)
circle1 = pyquadtree.Circle(6.0, 7.0, 2.0)

# 🖼️ Create a bounding rectangle for the quadtree
bounding_box = pyquadtree.Rectangle(0.0, 0.0, 10.0, 10.0)

# 🌳 Create a QuadTree with the given bounding box
quadtree = pyquadtree.QuadTree(bounding_box)

# ➕ Insert shapes into the quadtree with associated values
quadtree.insert(1, rect1)
quadtree.insert(2, circle1)

# 🔍 Check for collisions with a given shape
collisions_with_rect = quadtree.collisions(rect1)
print("Collisions with rect1:", collisions_with_rect)  # Output: [1, 2]

# 🔁 Relocate an existing shape in the quadtree
rect2 = pyquadtree.Rectangle(5.0, 5.0, 1.0, 1.0)
quadtree.relocate(1, rect2)

# 🔍 Check for collisions with a given shape after relocation
collisions_with_rect2 = quadtree.collisions(rect2)
print("Collisions with rect2:", collisions_with_rect2)  # Output: [1, 2]

# ❌ Delete a shape from the quadtree
quadtree.delete(2)

# 📋 Get all shapes in the quadtree
all_shapes = quadtree.all_shapes()
print("All shapes:", all_shapes)  # Output: [<Rectangle object>, ...]

# 📐 Get all node bounding boxes in the quadtree
all_bounding_boxes = quadtree.all_node_bounding_boxes()
print("All bounding boxes:", all_bounding_boxes)  # Output: [(0.0, 0.0, 10.0, 10.0), ...]
```
