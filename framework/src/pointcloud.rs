use anyhow::Result;
use glam::Vec3;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Basic element for the point cloud structure.
/// Four bytes of padding are inserted after Vec3 to keep the correct alignment when moving data to
/// the GPU (layout std430 required float3 to be 16-bytes aligned).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Point {
    pub position: Vec3,
    _padding0: [u8; 4],
    pub color: Vec3,
    _padding1: [u8; 4],
}

impl Point {
    pub fn new(position: Vec3, color: Vec3) -> Self {
        Point {
            position,
            _padding0: [0;4],
            color,
            _padding1: [0; 4],
        }
    }
}

/// A cloud of points, together with vectors containing minima and maxima in each direction.
pub struct PointCloud {
    pub points: Vec<Point>,
    pub minimum: Vec3,
    pub maximum: Vec3,
}

///
pub fn load_cloud(filename: &str) -> Result<PointCloud> {
    let file = File::open(filename)?;
    let mut lines = BufReader::new(file).lines();

    let mut points = Vec::new();
    let mut minimum: Vec3 = Vec3::MAX;
    let mut maximum: Vec3 = Vec3::MIN;

    while let Some(line) = lines.next() {
        let line = line?;
        let mut fields = line.split(' ');
        if let Some(start) = fields.next()
            && start == "v"
        {
            if let (Some(x), Some(y), Some(z)) = (fields.next(), fields.next(), fields.next()) {
                let x = x.parse::<f32>()?;
                let y = y.parse::<f32>()?;
                let z = z.parse::<f32>()?;
                let position = Vec3::new(x, y, z);

                let color = match (fields.next(), fields.next(), fields.next()) {
                    (Some(r), Some(g), Some(b)) => {
                        let r = r.parse::<f32>()?;
                        let g = g.parse::<f32>()?;
                        let b = b.parse::<f32>()?;
                        Vec3::new(r, g, b)
                    }
                    _ => Vec3::new(0.5, 0.5, 0.5),
                };

                minimum = minimum.min(position);
                maximum = maximum.max(position);
                points.push(Point::new(position, color));
            }
        }
    }
    Ok(PointCloud {
        points,
        minimum,
        maximum,
    })
}
