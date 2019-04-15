use brs::{chrono::prelude::*, uuid::Uuid};
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use std::collections::HashMap;
use std::io::{self, prelude::*};

pub use bl_save;
pub use brs;

// Keep this in sync. Would be nice to just determine the indices at compile time.
const FIXED_MATERIAL_TABLE: &[&str] = &["BMC_Plastic", "BMC_Glow", "BMC_Metallic"];
const BMC_PLASTIC: usize = 0;
const BMC_GLOW: usize = 1;
const BMC_METALLIC: usize = 2;

const BRICK_OWNER: usize = 0;

macro_rules! new_regex {
    ($e:expr) => {
        Regex::new($e).expect("failed to compile regex")
    };
}

lazy_static! {
    static ref BRICK_MAP_LITERAL: HashMap<&'static str, BrickMapping<'static>> = vec![
        ("1x1 Cone", BrickMapping::new("B_1x1_Cone")),
        ("1x1 Round", BrickMapping::new("B_1x1_Round")),
        ("1x1F Round", BrickMapping::new("B_1x1F_Round")),
        ("2x2 Round", BrickMapping::new("B_2x2_Round")),
        ("2x2F Round", BrickMapping::new("B_2x2F_Round")),
        (
            "Pine Tree",
            BrickMapping::with_offset("B_Pine_Tree", (0, 0, -6))
        ),
        ("32x32 Road", BrickMapping::with_size("PB_DefaultBrick", (160, 160, 2))),
        ("32x32 Road X", BrickMapping::with_size("PB_DefaultBrick", (160, 160, 2))),
        ("32x32 Road T", BrickMapping::with_size("PB_DefaultBrick", (160, 160, 2))),
        // ("1x4x5 Window", BrickMapping::with_size("PB_DefaultBrick", (4*5, 1*5, 5*6))),
        ("Music Brick", BrickMapping::with_size("PB_DefaultBrick", (5, 5, 6))),
    ]
    .into_iter()
    .collect();
    static ref BRICK_MAP_REGEX: Vec<(
        Regex,
        Box<dyn Fn(Captures) -> Option<BrickMapping<'static>> + Sync>
    )> = vec![
        (
            // TODO: Consider trying to handle fractional sizes that sometimes occur
            // TODO: Remove (?: Print)? when prints exist
            new_regex!(r"^(\d+)x(\d+)(?:x(\d+)|(F))?(?: Print)?$"),
            Box::new(|captures| {
                let width: u32 = captures.get(1).unwrap().as_str().parse().ok()?;
                let length: u32 = captures.get(2).unwrap().as_str().parse().ok()?;
                let z: u32 = if captures.get(4).is_some() { // F
                    2
                } else { // x(Z)
                    captures
                        .get(3)
                        .map(|g| g.as_str().parse::<u32>().ok())
                        .unwrap_or(Some(1))?
                        * 6
                };
                Some(BrickMapping::with_size(
                    "PB_DefaultBrick",
                    (width * 5, length * 5, z),
                ))
            })
        ),
        (
            // TODO: Remove (?: Print)? when prints exist
            new_regex!(r"^(-)?(25|45|72|80)° (Inv )?Ramp(?: (\d+)x)?( Corner)?(?: Print)?$"),
            Box::new(|captures| {
                let neg = captures.get(1).is_some();
                let inv = captures.get(3).is_some();
                let corner = captures.get(5).is_some();

                if inv && !corner {
                    return None;
                }

                let asset = if neg {
                    if inv {
                        "PB_DefaultRampInnerCornerInverted"
                    } else if corner {
                        "PB_DefaultRampCornerInverted"
                    } else {
                        "PB_DefaultRampInverted"
                    }
                } else if inv {
                    "PB_DefaultRampInnerCorner"
                } else if corner {
                    "PB_DefaultRampCorner"
                } else {
                    "PB_DefaultRamp"
                };

                let degree_str = captures.get(2).unwrap().as_str();

                let (x, z) = if degree_str == "25" {
                    (15, 6)
                } else if degree_str == "45" {
                    (10, 6)
                } else if degree_str == "72" {
                    (10, 18)
                } else if degree_str == "80" {
                    (10, 30)
                } else {
                    return None;
                };

                let mut y = x;

                if let Some(group) = captures.get(4) {
                    if corner {
                        return None;
                    }

                    let length: u32 = group.as_str().parse().ok()?;
                    y = length * 5;
                }

                Some(BrickMapping::with_size(asset, (x, y, z)))
            })
        ),
        (
            new_regex!(r"^(\d+)x(\d+)F Tile$"),
            Box::new(|captures| {
                let width: u32 = captures.get(1).unwrap().as_str().parse().ok()?;
                let length: u32 = captures.get(2).unwrap().as_str().parse().ok()?;
                Some(BrickMapping::with_size(
                    "PB_DefaultTile",
                    (width * 5, length * 5, 2),
                ))
            })
        ),
        (
            new_regex!(r"^(\d+)x(\d+) Base$"),
            Box::new(|captures| {
                let width: u32 = captures.get(1).unwrap().as_str().parse().ok()?;
                let length: u32 = captures.get(2).unwrap().as_str().parse().ok()?;
                Some(BrickMapping::with_size(
                    "PB_DefaultBrick",
                    (width * 5, length * 5, 2),
                ))
            })
        ),
        (
            new_regex!(r"^(\d+)x Cube$"),
            Box::new(|captures| {
                let size: u32 = captures.get(1).unwrap().as_str().parse().ok()?;
                Some(BrickMapping::with_size(
                    "PB_DefaultBrick",
                    (size * 5, size * 5, size * 5),
                ))
            })
        )
    ];
}

pub struct ConvertReport {
    pub write_data: brs::WriteData,
    pub unknown_ui_names: HashMap<String, usize>,
    pub count_success: usize,
    pub count_failure: usize,
}

pub fn convert(reader: bl_save::Reader<impl BufRead>) -> io::Result<ConvertReport> {
    let data = brs::WriteData {
        map: String::from("Unknown"),
        author: brs::User {
            id: Uuid::nil(),
            name: String::from("Unknown"),
        },
        description: reader.description().join("\n"),
        save_time: Utc::now(),
        mods: vec![],
        brick_assets: vec![],
        colors: reader.colors().iter().map(|c| map_color(*c)).collect(),
        materials: FIXED_MATERIAL_TABLE
            .iter()
            .map(|s| String::from(*s))
            .collect(),
        brick_owners: vec![brs::User {
            id: Uuid::from_bytes([u8::max_value(); 16]),
            name: String::from("PUBLIC"),
        }],
        bricks: Vec::with_capacity(reader.brick_count().min(10_000_000)),
    };

    let mut converter = Converter {
        write_data: data,
        ui_name_map: HashMap::new(),
        asset_map: HashMap::new(),
        unknown_ui_names: HashMap::new(),
    };

    let mut count_success = 0;
    let mut count_failure = 0;

    for from in reader {
        let from = from?;
        let mapping = converter.map_ui_name(&from.base.ui_name);

        match mapping {
            Some(BrickMapping::Direct {
                asset,
                size,
                offset,
            }) => {
                count_success += 1;

                let asset = *asset;
                let size = *size;
                let offset = *offset;

                let asset_name_index = converter.asset(asset);

                let position = (
                    (from.base.position.1 * 20.0) as i32 + offset.0,
                    (from.base.position.0 * 20.0) as i32 + offset.1,
                    (from.base.position.2 * 20.0) as i32 + offset.2,
                );

                let mut rotation = (from.base.angle + 1) % 4;

                if asset.find("Ramp").is_some() {
                    // rotation = ((rotation + 4 - 1) % 4) - 4;
                    rotation = (rotation + 4 - 1) % 4;
                }

                let material_index = match from.base.color_fx {
                    3 => BMC_GLOW,
                    1 | 2 => BMC_METALLIC,
                    _ => BMC_PLASTIC,
                };

                let brick = brs::Brick {
                    asset_name_index: asset_name_index as u32,
                    size,
                    position,
                    direction: brs::DIRECTION_Z_POSITIVE,
                    rotation,
                    collision: from.base.collision,
                    visibility: from.base.rendering,
                    material_index: material_index as u32,
                    color: brs::ColorMode::Set(u32::from(from.base.color_index)),
                    owner_index: BRICK_OWNER as u32,
                };

                converter.write_data.bricks.push(brick);
            }
            None => count_failure += 1,
        }
    }

    Ok(ConvertReport {
        write_data: converter.write_data,
        unknown_ui_names: converter.unknown_ui_names,
        count_success,
        count_failure,
    })
}

struct Converter {
    write_data: brs::WriteData,
    ui_name_map: HashMap<String, Option<BrickMapping<'static>>>,
    asset_map: HashMap<String, usize>,
    unknown_ui_names: HashMap<String, usize>,
}

impl Converter {
    fn map_ui_name(&mut self, ui_name: &str) -> Option<&BrickMapping<'static>> {
        // TODO: Don't clone the `ui_name` so much in here.
        let option = self
            .ui_name_map
            .entry(ui_name.to_string())
            .or_insert_with(|| {
                let mapping = map_ui_name(ui_name);
                if cfg!(debug_assertions) {
                    println!("mapped '{}' to {:?}", ui_name, mapping);
                }
                mapping
            })
            .as_ref();

        if option.is_none() {
            *self
                .unknown_ui_names
                .entry(ui_name.to_string())
                .or_default() += 1;
        }

        option

        /*
        if let Some(mapping) = self.ui_name_map.get(ui_name) {
            return mapping.as_ref();
        }

        let mapping = map_ui_name(ui_name);
        self.ui_name_map.insert(ui_name.to_string(), mapping);
        (&self.ui_name_map[ui_name]).as_ref()
        */
    }

    fn asset(&mut self, asset_name: &str) -> usize {
        if let Some(index) = self.asset_map.get(asset_name) {
            return *index;
        }

        let index = self.write_data.brick_assets.len();
        self.write_data.brick_assets.push(asset_name.to_string());
        self.asset_map.insert(asset_name.to_string(), index);

        index
    }
}

fn map_ui_name(ui_name: &str) -> Option<BrickMapping<'static>> {
    if let Some(mapping) = BRICK_MAP_LITERAL.get(ui_name) {
        return Some(mapping.clone());
    }

    for (regex, func) in BRICK_MAP_REGEX.iter() {
        if let Some(captures) = regex.captures(ui_name) {
            return func(captures);
        }
    }

    None
}

#[derive(Debug, Clone)]
enum BrickMapping<'s> {
    Direct {
        asset: &'s str,
        size: (u32, u32, u32),
        offset: (i32, i32, i32),
    },
}

impl<'s> BrickMapping<'s> {
    const fn new(asset: &'s str) -> Self {
        BrickMapping::Direct {
            asset,
            size: (0, 0, 0),
            offset: (0, 0, 0),
        }
    }

    const fn with_size(asset: &'s str, size: (u32, u32, u32)) -> Self {
        BrickMapping::Direct {
            asset,
            size,
            offset: (0, 0, 0),
        }
    }

    const fn with_offset(asset: &'s str, offset: (i32, i32, i32)) -> Self {
        BrickMapping::Direct {
            asset,
            size: (0, 0, 0),
            offset,
        }
    }

    const fn with_size_and_offset(
        asset: &'s str,
        size: (u32, u32, u32),
        offset: (i32, i32, i32),
    ) -> Self {
        BrickMapping::Direct {
            asset,
            size,
            offset,
        }
    }
}

fn map_color((r, g, b, a): (f32, f32, f32, f32)) -> brs::Color {
    // Convert into Unreal color space
    let r = r.powf(2.2);
    let g = g.powf(2.2);
    let b = b.powf(2.2);
    let a = a.powf(2.2);

    // Convert to 0-255
    let r = (r * 255.0).max(0.0).min(255.0) as u8;
    let g = (g * 255.0).max(0.0).min(255.0) as u8;
    let b = (b * 255.0).max(0.0).min(255.0) as u8;
    let a = (a * 255.0).max(0.0).min(255.0) as u8;

    brs::Color::from_rgba(r, g, b, a)
}
