use crate::Device;

pub struct MotorState {
    pub position: f32,
    velocity: f32,
}

pub enum MotorCommand {
    Move { target: f32 },
    Stop,
    Home,
}

pub struct MotorConfig {
    pub max_vel: f32,
}

impl Device for MotorConfig {
    fn tick(&self) {}
    fn name(&self) -> &str {
        "motor"
    }
    fn my_mom(&self) -> u32 {
        2 as u32
    }
}
