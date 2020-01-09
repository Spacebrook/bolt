use rand::prelude::*;

use {QuadTree, Shape, Rectangle, Circle};

#[test]
fn test_single_collision() {
    let mut qt = QuadTree::new(Rectangle {
        x: 0.,
        y: 0.,
        width: 100.,
        height: 100.,
    });
    qt.insert(0, Shape::Rectangle(Rectangle {
        x: 0.,
        y: 15.,
        width: 100.,
        height: 50.,
    }));
    let collisions = qt.collisions(Shape::Rectangle(Rectangle {
        x: 0.,
        y: 0.,
        width: 20.,
        height: 20.,
    }));

    assert_eq!(collisions, [0]);
}

#[test]
fn test_full_tree() {
    let mut qt = QuadTree::new(Rectangle {
        x: 0.,
        y: 0.,
        width: 1000.,
        height: 1000.,
    });

    qt.insert(0, Shape::Rectangle(Rectangle {
        x: 500.,
        y: 500.,
        width: 50.,
        height: 50.,
    }));
    qt.insert(1, Shape::Circle(Circle {
        x: 500.,
        y: 500.,
        radius: 25.,
    }));

    let mut rng = thread_rng();
    for i in 2..500 {
        qt.insert(i, Shape::Rectangle(Rectangle {
            x: rng.gen_range(0., 1000.),
            y: rng.gen_range(0., 1000.),
            width: rng.gen_range(0., 1000.),
            height: rng.gen_range(0., 1000.),
        }));
    }
    for i in 501..1000 {
        qt.insert(i, Shape::Circle(Circle {
            x: rng.gen_range(0., 1000.),
            y: rng.gen_range(0., 1000.),
            radius: rng.gen_range(0., 1000.),
        }));
    }
    let collisions = qt.collisions(Shape::Rectangle(Rectangle {
        x: 500.,
        y: 500.,
        width: 1.,
        height: 1.,
    }));

    assert!(collisions.contains(&0));
    assert!(collisions.contains(&1));

    let collisions = qt.collisions(Shape::Circle(Circle {
        x: 500.,
        y: 500.,
        radius: 1.,
    }));

    assert!(collisions.contains(&0));
    assert!(collisions.contains(&1));
}
