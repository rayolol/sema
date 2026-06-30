pub mod motor;

#[derive(Debug, Clone)]
pub struct TopLevelState {
    pub position: f32,
    pub velocity: f32,
}

pub trait Device {
    fn tick(&self);
    fn name(&self) -> &str;
    fn my_mom(&self) -> u32;
}
