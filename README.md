# 🌳 Bolt
High-performance Physics implementation in Rust, built for evades.io. This library provides a Python binding for easy integration and use in Python projects. The quadtree data structure is useful for spatial partitioning and efficient collision detection.

With Quadtree, you can effortlessly manage spatial data and efficiently perform collision detection. Happy coding! 🚀

## 🦀 Usage in Rust
Here's an example of how to use the quadtree library in Rust:

```rust
use quadtree::quadtree::QuadTree;
use quadtree::shapes::{Shape, Rectangle, Circle};

fn main() {
    // 🖼️ Create a bounding rectangle for the quadtree
    let bounding_box = Rectangle {
        x: 0.0,
        y: 0.0,
        width: 10.0,
        height: 10.0,
    };

    // 🌳 Create a QuadTree with the given bounding box
    let mut quadtree = QuadTree::new(bounding_box);

    // 🟩 Create instances of Rectangle and Circle
    let rect1 = Rectangle {
        x: 2.0,
        y: 3.0,
        width: 4.0,
        height: 5.0,
    };
    let circle1 = Circle::new(6.0, 7.0, 2.0);

    // ➕ Insert shapes into the quadtree with associated values
    quadtree.insert(1, Box::new(rect1));
    quadtree.insert(2, Box::new(circle1));

    // 🔍 Check for collisions with a given shape
    let mut collisions_with_rect = Vec::new();
    quadtree.collisions(&rect1, &mut collisions_with_rect);
    println!("Collisions with rect1: {:?}", collisions_with_rect);  // Output: [1, 2]

    // 🔁 Relocate an existing shape in the quadtree
    let rect2 = Rectangle {
        x: 5.0,
        y: 5.0,
        width: 1.0,
        height: 1.0,
    };
    quadtree.relocate(1, Box::new(rect2));

    // 🔍 Check for collisions with a given shape after relocation
    let mut collisions_with_rect2 = Vec::new();
    quadtree.collisions(&rect2, &mut collisions_with_rect2);
    println!("Collisions with rect2: {:?}", collisions_with_rect2);  // Output: [1, 2]

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
}
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
