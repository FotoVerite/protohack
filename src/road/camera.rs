#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub road: u16,
    pub location: u16,
    pub limit: u16,
}

impl Camera {
    pub fn new(road: u16, location: u16, limit: u16) -> Self {
        Self {
            road,
            location,
            limit,
        }
    }

    pub fn speeding(self, own_timestamp: &u32, other: &Camera, other_timestamp: &u32) -> Option<u16> {
        let distance_miles = (other.location as i32 - self.location as i32).abs() as f64;
        let duration_seconds = (other_timestamp - own_timestamp) as f64;
        let speed_mph = distance_miles / (duration_seconds / 3600.0);
        println!("Self limit: {}, Other limit: {}, Speed: {}", self.limit, other.limit, speed_mph);
        if speed_mph > self.limit as f64 {
            let speed_rounded_mph = speed_mph.round() as u16;
            println!("Car found speeding for camera {:?} at speed {}", self, speed_rounded_mph);
            return Some(speed_rounded_mph)
        }
        return None
    }
}
