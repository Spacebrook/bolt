from typing import List, Tuple, Optional, Any, Sequence, TypeAlias
from typing_extensions import Protocol

class ShapeProtocol(Protocol):
    pass

class Circle(ShapeProtocol):
    x: float
    y: float
    radius: float
    def __init__(self, x: float, y: float, radius: float) -> None: ...

class Rectangle(ShapeProtocol):
    def __init__(self, x: float, y: float, width: float, height: float) -> None: ...
    @property
    def x(self) -> float: ...
    @x.setter
    def x(self, value: float) -> None: ...
    @property
    def y(self) -> float: ...
    @y.setter
    def y(self, value: float) -> None: ...
    @property
    def width(self) -> float: ...
    @property
    def height(self) -> float: ...
    def left(self) -> float: ...
    def right(self) -> float: ...
    def top(self) -> float: ...
    def bottom(self) -> float: ...
    def top_left(self) -> Tuple[float, float]: ...
    def top_right(self) -> Tuple[float, float]: ...
    def bottom_left(self) -> Tuple[float, float]: ...
    def bottom_right(self) -> Tuple[float, float]: ...
    def distance_to_point(self, x: float, y: float) -> float: ...
    def contains_circle(self, x: float, y: float, radius: float) -> bool: ...
    def contains_point(self, x: float, y: float) -> bool: ...
    def expand_to_include(self, other: 'Rectangle') -> None: ...
    def get_random_circle_coords_inside(self, radius: float, rng: 'Rng') -> Tuple[float, float]: ...
    def copy(self) -> 'Rectangle': ...

class Rng:
    def __init__(self) -> None: ...
    def seed_rng(self, seed: int) -> None: ...

class Config:
    def __init__(self, pool_size: int, node_capacity: int, max_depth: int) -> None: ...

class QuadTree:
    def __init__(self, bounding_box: Rectangle) -> None: ...
    @staticmethod
    def new_with_config(bounding_box: Rectangle, config: Config) -> 'QuadTree': ...
    def insert(self, value: int, shape: ShapeProtocol, entity_type: Optional[int] = None) -> None: ...
    def delete(self, value: int) -> None: ...
    def collisions(self, shape: ShapeProtocol) -> List[int]: ...
    def collisions_filter(self, shape: ShapeProtocol, entity_types: Optional[List[int]] = None) -> List[int]: ...
    def collisions_batch(self, shapes: List[ShapeProtocol]) -> List[List[int]]: ...
    def collisions_batch_filter(self, shapes: List[ShapeProtocol], entity_types: Optional[List[int]] = None) -> List[List[int]]: ...
    def relocate(self, value: int, shape: ShapeProtocol, entity_type: Optional[int] = None) -> None: ...
    def relocate_batch(self, relocation_requests: List[Tuple[int, ShapeProtocol, Optional[int]]]) -> None: ...
    def all_node_bounding_boxes(self) -> List[Tuple[float, float, float, float]]: ...
    def all_shapes(self) -> List[ShapeProtocol]: ...

class collisions:
    @staticmethod
    def get_mtv(entity: ShapeProtocol, colliding_polys: Sequence[ShapeProtocol]) -> Optional[Tuple[float, float]]: ...

class quadtree:
    Config: TypeAlias = Config
    QuadTree: TypeAlias = QuadTree

class serialization:
    class DiffFieldSet:
        def __init__(self, field_types: List[int], field_defaults: List[Any]) -> None: ...
        def update(self, updates: List[Any]) -> None: ...
        def has_changed(self) -> bool: ...
        def get_diff(self) -> List[Tuple[int, Any]]: ...
        def get_all(self) -> List[Tuple[int, Any]]: ...
