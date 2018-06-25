
#[derive(Clone, Debug)]
pub struct Position {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}


#[derive(Clone, Debug)]
pub struct PlayerState {
	pub x: f64,
	pub y: f64,
	pub z: f64,
	pub yaw: f32,
	pub pitch: f32,
	pub on_ground: bool,
}