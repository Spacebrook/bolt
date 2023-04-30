use common::shapes::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

#[test]
fn test_new_and_getters() {
    let rect = Rectangle::new(2.0, 3.0, 4.0, 6.0);
    assert_eq!(rect.width(), 4.0);
    assert_eq!(rect.height(), 6.0);
    assert_eq!(rect.left(), 0.0);
    assert_eq!(rect.right(), 4.0);
    assert_eq!(rect.top(), 0.0);
    assert_eq!(rect.bottom(), 6.0);
    assert_eq!(rect.top_left(), (0.0, 0.0));
    assert_eq!(rect.top_right(), (4.0, 0.0));
    assert_eq!(rect.bottom_left(), (0.0, 6.0));
    assert_eq!(rect.bottom_right(), (4.0, 6.0));
}

#[test]
fn test_contains_point_center() {
    let rect = Rectangle::new(0.0, 0.0, 4.0, 6.0);
    assert!(rect.contains_point(0.0, 0.0));
}

#[test]
fn test_distance_to_point() {
    let rect = Rectangle::new(2.0, 3.0, 4.0, 6.0);
    assert_eq!(rect.distance_to_point(2.0, 3.0), 0.0);
    assert_eq!(rect.distance_to_point(6.0, 3.0), 4.0);
    assert_eq!(rect.distance_to_point(2.0, 8.0), 4.0);
}

#[test]
fn test_contains_circle() {
    let rect = Rectangle::new(2.0, 3.0, 4.0, 6.0);
    assert!(rect.contains_circle(2.0, 3.0, 1.0));
    assert!(!rect.contains_circle(6.0, 3.0, 1.0));
    assert!(!rect.contains_circle(2.0, 8.0, 1.0));
}

#[test]
fn test_contains_point() {
    let rect = Rectangle::new(2.0, 3.0, 4.0, 6.0);
    assert!(rect.contains_point(2.0, 3.0));
    assert!(!rect.contains_point(6.0, 3.0));
    assert!(!rect.contains_point(2.0, 8.0));
}

#[test]
fn test_expand_to_include() {
    let mut rect = Rectangle::new(2.0, 3.0, 4.0, 6.0);
    let other_rect = Rectangle::new(6.0, 5.0, 4.0, 2.0);
    rect.expand_to_include(&other_rect);
    assert_eq!(rect.width(), 8.0);
    assert_eq!(rect.height(), 6.0);
    assert_eq!(rect.left(), 0.0);
    assert_eq!(rect.right(), 8.0);
    assert_eq!(rect.top(), 0.0);
    assert_eq!(rect.bottom(), 6.0);
}
#[test]
fn test_get_random_circle_coords_inside() {
    let rect = Rectangle::new(2.0, 3.0, 6.0, 8.0);
    let radius = 1.0;

    // Use a fixed seed for reproducibility.
    let mut rng: StdRng = SeedableRng::seed_from_u64(123);

    for _ in 0..10 {
        let (x, y) = rect.get_random_circle_coords_inside(radius, &mut rng);
        assert!(rect.contains_circle(x, y, radius));
    }
}

#[test]
fn test_get_random_circle_coords_inside_small_rectangle() {
    let rect = Rectangle::new(2.0, 3.0, 2.0, 2.0);
    let radius = 2.0;

    // Use a fixed seed for reproducibility.
    let mut rng: StdRng = SeedableRng::seed_from_u64(123);

    let (x, y) = rect.get_random_circle_coords_inside(radius, &mut rng);
    // The generated coordinates should be clamped to the left/top of the rectangle.
    assert_eq!(x, rect.left() + radius + 1.0);
    assert_eq!(y, rect.top() + radius + 1.0);
}
