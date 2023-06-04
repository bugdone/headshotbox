use serde_json::json;

const EPSILON: f64 = 0.0001;
const PLAYER_HEIGHT: f64 = 72_f64;
const PLAYER_CROUCH_HEIGHT: f64 = 50_f64;
const SMOKE_RADIUS: f64 = 140_f64;
const SMOKE_HEIGHT: f64 = 130_f64;

#[derive(Clone, Copy)]
pub(crate) struct Point {
    x: f64,
    y: f64,
    z: f64,
}

impl Point {
    pub(crate) fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

impl From<Point> for serde_json::Value {
    fn from(p: Point) -> Self {
        json!([p.x, p.y, p.z])
    }
}

impl std::ops::Sub for Point {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

pub(crate) fn through_smoke(p1: &Point, p2: &Point, smoke: &Point) -> bool {
    let killer = Point::new(p1.x, p1.y, p1.z + PLAYER_CROUCH_HEIGHT);
    // Check if shooting to the legs AND head of the victim goes through smoke
    intersects(killer, *p2, *smoke, SMOKE_RADIUS, SMOKE_HEIGHT)
        && intersects(
            killer,
            Point::new(p2.x, p2.y, p2.z + PLAYER_HEIGHT),
            *smoke,
            SMOKE_RADIUS,
            SMOKE_HEIGHT,
        )
}

fn intersects(line1: Point, line2: Point, center: Point, radius: f64, height: f64) -> bool {
    let start = line1 - center;
    let direction = line2 - line1;
    let start = Point::new(start.x / radius, start.y / radius, start.z / height);
    let direction = Point::new(
        direction.x / radius,
        direction.y / radius,
        direction.z / height,
    );
    let a = direction.x * direction.x + direction.y * direction.y;
    let b = 2_f64 * start.x * direction.x + 2_f64 * start.y * direction.y;
    let c = start.x * start.x + start.y * start.y - 1_f64;

    let b24ac = b * b - 4_f64 * a * c;
    if b24ac < 0_f64 {
        return false;
    }

    let sqb24ac = b24ac.sqrt();
    let mut t0 = (-b + sqb24ac) / (2_f64 * a);
    let mut t1 = (-b - sqb24ac) / (2_f64 * a);
    if t0 > t1 {
        std::mem::swap(&mut t0, &mut t1);
    }

    let y0 = start.z + t0 * direction.z;
    let y1 = start.z + t1 * direction.z;

    if y0 < -1_f64 {
        if y1 < -1_f64 {
            return false;
        } else {
            // hit the cap
            let th = t0 + (t1 - t0) * (y0 + 1_f64) / (y0 - y1);
            if th <= -EPSILON {
                return false;
            }
            return true;
        }
    } else if (-1_f64..=1_f64).contains(&y0) {
        // hit the cylinder bit
        // check if on the segment
        if !(0_f64..=1_f64).contains(&t0) {
            return false;
        }
        if t0 < -EPSILON {
            return false;
        }
        return true;
    } else if y0 > 1_f64 {
        if y1 > 1_f64 {
            return false;
        } else {
            // hit the cap
            let th = t0 + (t1 - t0) * (y0 - 1_f64) / (y0 - y1);
            if th <= 0_f64 {
                return false;
            }
            return true;
        }
    }
    false
}
