use brs::{chrono::prelude::*, uuid::Uuid};
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use std::{
    collections::{HashMap, HashSet},
    io::{self, prelude::*},
    ops::Neg,
};

pub use bl_save;
pub use brs;

// Keep this in sync. Would be nice to just determine the indices at compile time.
const FIXED_MATERIAL_TABLE: &[&str] = &["BMC_Plastic", "BMC_Glow", "BMC_Metallic"];
const BMC_PLASTIC: usize = 0;
const BMC_GLOW: usize = 1;
const BMC_METALLIC: usize = 2;

const BRICK_OWNER: usize = 0;

macro_rules! map {
    [$($key:expr => $value:expr),* $(,)?] => {
        vec![
            $(
                ($key, $value),
            )*
        ].into_iter().collect()
    }
}

macro_rules! brick_map_literal {
    [$($ui:expr => $map:expr),* $(,)?] => {
        map![
            $($ui => AsBrickMappingVec::as_brick_mapping_vec($map),)*
        ]
    }
}

macro_rules! brick_map_regex {
    [$($source:expr => $func:expr),* $(,)?] => {
        vec![
            $(
                (
                    Regex::new($source).expect("failed to compile regex"),
                    Box::new($func),
                ),
            )*
        ]
    }
}

type RegexHandler =
    Box<dyn Fn(Captures, &bl_save::Brick) -> Option<Vec<BrickMapping<'static>>> + Sync>;

lazy_static! {
    static ref BLANK_PRINTS: HashSet<&'static str> = vec![
        "Letters/-space",
        "1x2f/blank",
        "2x2f/blank",
    ].into_iter().collect();

    static ref BRICK_ROAD_LANE: BrickMapping<'static> = BrickMapping::new("PB_DefaultTile")
        .color_override(brs::Color::from_rgba(51, 51, 51, 255));
    static ref BRICK_ROAD_STRIPE: BrickMapping<'static> = BrickMapping::new("PB_DefaultTile")
        .color_override(brs::Color::from_rgba(254, 254, 232, 255));

    static ref BRICK_MAP_LITERAL: HashMap<&'static str, Vec<BrickMapping<'static>>> = brick_map_literal![
        "1x1 Cone" => BrickMapping::new("B_1x1_Cone"),
        "1x1 Round" => BrickMapping::new("B_1x1_Round"),
        "1x1F Round" => BrickMapping::new("B_1x1F_Round"),
        "2x2 Round" => BrickMapping::new("B_2x2_Round"),
        "2x2F Round" => BrickMapping::new("B_2x2F_Round"),
        "Pine Tree" => BrickMapping::new("B_Pine_Tree").offset((0, 0, -6)),

        // "1x4x5 Window" => BrickMapping::new("PB_DefaultBrick").size((4*5, 1*5, 5*6)),
        "Music Brick" => BrickMapping::new("PB_DefaultBrick").size((5, 5, 6)),
        "2x2 Disc" => BrickMapping::new("B_2x2F_Round"),

        "32x32 Road" => vec![
            // left and right sidewalks
            BrickMapping::new("PB_DefaultBrick").size((9*5, 32*5, 2)).offset((0, -115, 0)),
            BrickMapping::new("PB_DefaultBrick").size((9*5, 32*5, 2)).offset((0, 115, 0)),
            // left and right stripes
            BRICK_ROAD_STRIPE.clone().size((1*5, 32*5, 2)).offset((0, -65, 0)),
            BRICK_ROAD_STRIPE.clone().size((1*5, 32*5, 2)).offset((0, 65, 0)),
            // lanes
            BRICK_ROAD_LANE.clone().size((6*5, 32*5, 2)).offset((0, -6*5, 0)),
            BRICK_ROAD_LANE.clone().size((6*5, 32*5, 2)).offset((0, 6*5, 0)),
        ],

        // Orientations are relative to this camera position on Beta City:
        // 39.5712 0.0598862 14.5026 0.999998 -0.0007625 0.00180403 0.799784
        "32x32 Road T" => vec![
            BrickMapping::new("PB_DefaultBrick").size((9*5, 32*5, 2)).offset((0, -115, 0)), // top
            BrickMapping::new("PB_DefaultBrick").size((9*5, 9*5, 2)).offset((-115, 115, 0)), // bottom left
            BrickMapping::new("PB_DefaultBrick").size((9*5, 9*5, 2)).offset((115, 115, 0)), // bottom right
            BRICK_ROAD_STRIPE.clone().size((1*5, 32*5, 2)).offset((0, -65, 0)), // straight top
            BRICK_ROAD_STRIPE.clone().size((1*5, 32*5, 2)).offset((0, 65, 0)), // straight bottom
            BRICK_ROAD_STRIPE.clone().size((1*5, 9*5, 2)).rotation_offset(0).offset((-13*5, 23*5, 0)), // bottom left
            BRICK_ROAD_STRIPE.clone().size((1*5, 9*5, 2)).rotation_offset(0).offset((13*5, 23*5, 0)), // bottom right
            BRICK_ROAD_LANE.clone().size((6*5, 32*5, 2)).offset((0, -6*5, 0)), // straight top
            BRICK_ROAD_LANE.clone().size((6*5, 32*5, 2)).offset((0, 6*5, 0)), // straight bottom
            BRICK_ROAD_LANE.clone().size((6*5, 9*5, 2)).rotation_offset(0).offset((-6*5, 23*5, 0)), // bottom left
            BRICK_ROAD_LANE.clone().size((6*5, 9*5, 2)).rotation_offset(0).offset((6*5, 23*5, 0)), // bottom right
        ],

        // Orientations are relative to this camera position on Beta City:
        // -56.5 -35 4 0 0 1 3.14159
        "32x32 Road X" => vec![
            BrickMapping::new("PB_DefaultBrick").size((9*5, 9*5, 2)).offset((-23*5, -23*5, 0)), // top left
            BrickMapping::new("PB_DefaultBrick").size((9*5, 9*5, 2)).offset((23*5, -23*5, 0)), // top right
            BrickMapping::new("PB_DefaultBrick").size((9*5, 9*5, 2)).offset((-23*5, 23*5, 0)), // bottom left
            BrickMapping::new("PB_DefaultBrick").size((9*5, 9*5, 2)).offset((23*5, 23*5, 0)), // bottom right
            BRICK_ROAD_STRIPE.clone().size((1*5, 1*5, 2)).offset((13*5, -13*5, 0)), // corner top left
            BRICK_ROAD_STRIPE.clone().size((1*5, 1*5, 2)).offset((13*5, 13*5, 0)), // corner right right
            BRICK_ROAD_STRIPE.clone().size((1*5, 1*5, 2)).offset((-13*5, -13*5, 0)), // corner bottom left
            BRICK_ROAD_STRIPE.clone().size((1*5, 1*5, 2)).offset((-13*5, 13*5, 0)), // corner bottom right
            BRICK_ROAD_STRIPE.clone().size((1*5, 12*5, 2)).rotation_offset(0).offset((-13*5, 0, 0)), // inner bottom
            BRICK_ROAD_STRIPE.clone().size((1*5, 12*5, 2)).rotation_offset(0).offset((13*5, 0, 0)), // inner top
            BRICK_ROAD_STRIPE.clone().size((1*5, 12*5, 2)).offset((0, -13*5, 0)), // inner left
            BRICK_ROAD_STRIPE.clone().size((1*5, 12*5, 2)).offset((0, 13*5, 0)), // inner right
            BRICK_ROAD_STRIPE.clone().size((1*5, 9*5, 2)).rotation_offset(0).offset((-13*5, 23*5, 0)), // right bottom
            BRICK_ROAD_STRIPE.clone().size((1*5, 9*5, 2)).rotation_offset(0).offset((13*5, 23*5, 0)), // right top
            BRICK_ROAD_STRIPE.clone().size((1*5, 9*5, 2)).rotation_offset(0).offset((-13*5, -23*5, 0)), // left bottom
            BRICK_ROAD_STRIPE.clone().size((1*5, 9*5, 2)).rotation_offset(0).offset((13*5, -23*5, 0)), // left top
            BRICK_ROAD_STRIPE.clone().size((1*5, 9*5, 2)).offset((-23*5, -13*5, 0)), // bottom left
            BRICK_ROAD_STRIPE.clone().size((1*5, 9*5, 2)).offset((-23*5, 13*5, 0)), // bottom right
            BRICK_ROAD_STRIPE.clone().size((1*5, 9*5, 2)).offset((23*5, -13*5, 0)), // top left
            BRICK_ROAD_STRIPE.clone().size((1*5, 9*5, 2)).offset((23*5, 13*5, 0)), // top right
            BRICK_ROAD_LANE.clone().size((6*5, 6*5, 2)).offset((-6*5, -6*5, 0)), // inner bottom left
            BRICK_ROAD_LANE.clone().size((6*5, 6*5, 2)).offset((-6*5, 6*5, 0)), // inner bottom right
            BRICK_ROAD_LANE.clone().size((6*5, 6*5, 2)).offset((6*5, -6*5, 0)), // inner top left
            BRICK_ROAD_LANE.clone().size((6*5, 6*5, 2)).offset((6*5, 6*5, 0)), // inner top right
            BRICK_ROAD_LANE.clone().size((6*5, 9*5, 2)).rotation_offset(0).offset((-6*5, 23*5, 0)), // right bottom
            BRICK_ROAD_LANE.clone().size((6*5, 9*5, 2)).rotation_offset(0).offset((6*5, 23*5, 0)), // right top
            BRICK_ROAD_LANE.clone().size((6*5, 9*5, 2)).rotation_offset(0).offset((-6*5, -23*5, 0)), // left bottom
            BRICK_ROAD_LANE.clone().size((6*5, 9*5, 2)).rotation_offset(0).offset((6*5, -23*5, 0)), // left top
            BRICK_ROAD_LANE.clone().size((6*5, 9*5, 2)).offset((-23*5, -6*5, 0)), // bottom left
            BRICK_ROAD_LANE.clone().size((6*5, 9*5, 2)).offset((-23*5, 6*5, 0)), // bottom right
            BRICK_ROAD_LANE.clone().size((6*5, 9*5, 2)).offset((23*5, -6*5, 0)), // top left
            BRICK_ROAD_LANE.clone().size((6*5, 9*5, 2)).offset((23*5, 6*5, 0)), // top right
        ],

        // Orientations are relative to this camera position on Beta City:
        // -25.9168 -110.523 12.5993 0.996034 0.0289472 -0.0841301 0.665224
        "32x32 Road C" => vec![
            // sidewalks
            BrickMapping::new("PB_DefaultBrick").size((9*5, 9*5, 2)).offset((-115, 115, 0)), // top left
            BrickMapping::new("PB_DefaultBrick").size((9*5, 9*5, 2)).offset((115, -115, 0)), // bottom right
            BrickMapping::new("PB_DefaultBrick").size((9*5, 23*5, 2)).rotation_offset(0).offset((115, 45, 0)), // bottom left
            BrickMapping::new("PB_DefaultBrick").size((9*5, 23*5, 2)).offset((-45, -115, 0)), // top right
            BRICK_ROAD_STRIPE.clone().size((1*5, 9*5, 2)).offset((-115, 65, 0)), // inner right
            BRICK_ROAD_STRIPE.clone().size((1*5, 9*5, 2)).rotation_offset(0).offset((-65, 115, 0)), // inner bottom
            BRICK_ROAD_STRIPE.clone().size((1*5, 22*5, 2)).offset((-50, -65, 0)), // top right
            BRICK_ROAD_STRIPE.clone().size((1*5, 22*5, 2)).rotation_offset(0).offset((65, 50, 0)), // bottom left
            BRICK_ROAD_STRIPE.clone().size((1*5, 1*5, 2)).offset((65, -65, 0)), // bottom right
            BRICK_ROAD_STRIPE.clone().size((1*5, 1*5, 2)).rotation_offset(0).offset((-65, 65, 0)), // inner bottom right
            BRICK_ROAD_LANE.clone().size((6*5, 10*5, 2)).offset((-22*5, 6*5, 0)), // top left
            BRICK_ROAD_LANE.clone().size((6*5, 16*5, 2)).offset((-16*5, -6*5, 0)), // top right
            BRICK_ROAD_LANE.clone().size((6*5, 16*5, 2)).rotation_offset(0).offset((6*5, 16*5, 0)), // bottom left
            BRICK_ROAD_LANE.clone().size((6*5, 10*5, 2)).rotation_offset(0).offset((-6*5, 22*5, 0)), // left top
            BRICK_ROAD_LANE.clone().size((6*5, 6*5, 2)).offset((-6*5, 6*5, 0)), // inner top left
            BRICK_ROAD_LANE.clone().size((6*5, 6*5, 2)).offset((6*5, -6*5, 0)), // inner bottom right
        ],
    ];

    static ref BRICK_MAP_REGEX: Vec<(Regex, RegexHandler)> = brick_map_regex![
        // TODO: Consider trying to handle fractional sizes that sometimes occur
        // TODO: Remove (?: Print)? when prints exist
        r"^(\d+)x(\d+)(?:x(\d+)|([Ff])|([Hh]))?( Print)?$" => |captures, from| {
            let width: u32 = captures.get(1).unwrap().as_str().parse().ok()?;
            let length: u32 = captures.get(2).unwrap().as_str().parse().ok()?;
            let z: u32 = if captures.get(4).is_some() { // F
                2
            } else if captures.get(5).is_some() { // H
                4
            } else { // x(Z)
                captures
                    .get(3)
                    .map(|g| g.as_str().parse::<u32>().ok())
                    .unwrap_or(Some(1))?
                    * 6
            };

            let print = captures.get(6).is_some();
            let asset = if print && BLANK_PRINTS.contains(from.base.print.as_str()) {
                "PB_DefaultTile"
            } else {
                "PB_DefaultBrick"
            };
            let rotation_offset = if print { 0 } else { 1 };

            Some(vec![BrickMapping::new(asset)
                .size((width * 5, length * 5, z))
                .rotation_offset(rotation_offset)])
        },

        // TODO: Remove (?: Print)? when prints exist
        r"^(-)?(25|45|72|80)° (Inv )?Ramp(?: (\d+)x)?( Corner)?(?: Print)?$" => |captures, _| {
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

            Some(vec![BrickMapping::new(asset).size((x, y, z)).rotation_offset(0)])
        },

        r"^(\d+)x(\d+)F Tile$" => |captures, _| {
            let width: u32 = captures.get(1).unwrap().as_str().parse().ok()?;
            let length: u32 = captures.get(2).unwrap().as_str().parse().ok()?;
            Some(vec![BrickMapping::new("PB_DefaultTile").size((width * 5, length * 5, 2))])
        },
        r"^(\d+)x(\d+) Base$" => |captures, _| {
            let width: u32 = captures.get(1).unwrap().as_str().parse().ok()?;
            let length: u32 = captures.get(2).unwrap().as_str().parse().ok()?;
            Some(vec![BrickMapping::new("PB_DefaultBrick").size((width * 5, length * 5, 2))])
        },
        r"^(\d+)x Cube$" => |captures, _| {
            let size: u32 = captures.get(1).unwrap().as_str().parse().ok()?;
            Some(vec![BrickMapping::new("PB_DefaultBrick").size((size * 5, size * 5, size * 5))])
        },
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
        asset_map: HashMap::new(),
        unknown_ui_names: HashMap::new(),
    };

    let mut count_success = 0;
    let mut count_failure = 0;

    for from in reader {
        let from = from?;
        let option = converter.map_brick(&from);

        let mappings = match option {
            Some(mappings) => {
                count_success += 1;
                mappings
            }
            None => {
                count_failure += 1;
                continue;
            }
        };

        for BrickMapping {
            asset,
            size,
            offset,
            rotation_offset,
            color_override,
        } in mappings
        {
            let asset_name_index = converter.asset(asset);
            let rotation = (from.base.angle + rotation_offset) % 4;

            let rotated_xy = rotate_offset((offset.0, offset.1), from.base.angle);
            let offset = (rotated_xy.0, rotated_xy.1, offset.2);

            let position = (
                (from.base.position.1 * 20.0) as i32 + offset.0,
                (from.base.position.0 * 20.0) as i32 + offset.1,
                (from.base.position.2 * 20.0) as i32 + offset.2,
            );

            let material_index = match from.base.color_fx {
                3 => BMC_GLOW,
                1 | 2 => BMC_METALLIC,
                _ => BMC_PLASTIC,
            };

            let color_index = match color_override {
                Some(ref color) => converter.color(color) as u32,
                None => u32::from(from.base.color_index),
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
                color: brs::ColorMode::Set(color_index),
                owner_index: BRICK_OWNER as u32,
            };

            converter.write_data.bricks.push(brick);
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
    asset_map: HashMap<String, usize>,
    unknown_ui_names: HashMap<String, usize>,
}

impl Converter {
    fn map_brick(&mut self, from: &bl_save::Brick) -> Option<Vec<BrickMapping<'static>>> {
        let mapping = map_brick(from);

        if cfg!(debug_assertions) {
            println!("mapped '{}' to {:?}", from.base.ui_name, mapping);
        }

        if mapping.is_none() {
            *self
                .unknown_ui_names
                .entry(from.base.ui_name.clone())
                .or_default() += 1;
        }

        mapping
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

    fn color(&mut self, color: &brs::Color) -> usize {
        // TODO: Optimize lookup with a map
        for (index, other) in self.write_data.colors.iter().enumerate() {
            if other == color {
                return index;
            }
        }

        let index = self.write_data.colors.len();
        self.write_data.colors.push(color.clone());
        index
    }
}

fn map_brick(from: &bl_save::Brick) -> Option<Vec<BrickMapping<'static>>> {
    let ui_name = from.base.ui_name.as_str();

    if let Some(mapping) = BRICK_MAP_LITERAL.get(ui_name) {
        return Some(mapping.clone());
    }

    for (regex, func) in BRICK_MAP_REGEX.iter() {
        if let Some(captures) = regex.captures(ui_name) {
            return func(captures, from);
        }
    }

    None
}

#[derive(Debug, Clone)]
struct BrickMapping<'s> {
    asset: &'s str,
    size: (u32, u32, u32),
    offset: (i32, i32, i32),
    rotation_offset: u8,
    color_override: Option<brs::Color>,
}

impl<'s> BrickMapping<'s> {
    const fn new(asset: &'s str) -> Self {
        Self {
            asset,
            size: (0, 0, 0),
            offset: (0, 0, 0),
            rotation_offset: 1,
            color_override: None,
        }
    }

    fn size(mut self, size: (u32, u32, u32)) -> Self {
        self.size = size;
        self
    }

    fn offset(mut self, offset: (i32, i32, i32)) -> Self {
        self.offset = offset;
        self
    }

    fn rotation_offset(mut self, rotation: u8) -> Self {
        self.rotation_offset = rotation;
        self
    }

    fn color_override(mut self, color_override: brs::Color) -> Self {
        self.color_override = Some(color_override);
        self
    }
}

trait AsBrickMappingVec<'s> {
    fn as_brick_mapping_vec(self) -> Vec<BrickMapping<'s>>;
}

impl<'s> AsBrickMappingVec<'s> for BrickMapping<'s> {
    fn as_brick_mapping_vec(self) -> Vec<BrickMapping<'s>> {
        vec![self]
    }
}

impl<'s> AsBrickMappingVec<'s> for Vec<BrickMapping<'s>> {
    fn as_brick_mapping_vec(self) -> Vec<BrickMapping<'s>> {
        self
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

fn rotate_offset(mut offset: (i32, i32), angle: u8) -> (i32, i32) {
    for _ in 0..angle {
        offset = rotate_90_2d(offset);
    }
    offset
}

fn rotate_90_2d<X, Y: Neg>((x, y): (X, Y)) -> (<Y as Neg>::Output, X) {
    (-y, x)
}
