
#[derive(Debug, Clone)]
pub struct Config {
    pub pool_size: usize,
    pub node_capacity: usize,
    pub max_depth: usize,
    pub min_size: f32,
    pub looseness: f32,
    pub large_entity_threshold_factor: f32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            pool_size: 4000,
            node_capacity: 64,
            max_depth: 6,
            min_size: 1.0,
            looseness: 1.0,
            large_entity_threshold_factor: 0.0,
        }
    }
}

#[derive(Clone)]
pub struct RelocationRequest {
    pub value: u32,
    pub shape: ShapeEnum,
    pub entity_type: Option<u32>,
}
