use bls2brs::{bl_save, brs, convert};
use std::{
    fs::File,
    io::{self, BufReader},
    path::{Path, PathBuf},
};

fn main() -> io::Result<()> {
    let args = parse_args().unwrap_or_else(|program| {
        eprintln!("Usage: {} <bls files ...>", program);
        std::process::exit(1);
    });

    for (i, input_path) in args.input_paths.iter().enumerate() {
        if i > 0 {
            println!();
        }

        let mut output_path = PathBuf::from(input_path);
        output_path.set_extension("brs");

        convert_one(input_path, &output_path);
    }

    Ok(())
}

fn convert_one(input_path: impl AsRef<Path>, output_path: impl AsRef<Path>) {
    let input_path = input_path.as_ref();
    let output_path = output_path.as_ref();

    println!("Converting {}", input_path.display());

    let input_file = try_or_die(File::open(input_path), "Failed to open bls file");
    let input_file = BufReader::new(input_file);
    let input_reader = try_or_die(bl_save::Reader::new(input_file), "Failed to read bls file");

    let mut converted = try_or_die(convert(input_reader), "Failed to convert bls file");

    if let Some(file_name) = input_path.file_name() {
        let mut prefix = format!(
            "Converted from {} with bls2brs.",
            file_name.to_string_lossy()
        );

        if !converted.write_data.description.is_empty() {
            prefix.push('\n');
        }

        converted.write_data.description.insert_str(0, &prefix);
    }

    if !converted.unknown_ui_names.is_empty() {
        println!("Unknown bricks:");
        let mut ui_names: Vec<_> = converted.unknown_ui_names.into_iter().collect();
        ui_names.sort_by(|(_, ac), (_, bc)| ac.cmp(bc).reverse());
        for (ui_name, count) in ui_names {
            let ui_name = if ui_name.ends_with(" ") {
                format!("{:?}", ui_name)
            } else {
                ui_name
            };
            println!("  {:<28} {:>4} bricks", ui_name, count);
        }
    }

    if converted.count_failure > 0 {
        println!("{} bricks failed to convert", converted.count_failure);
    }

    println!(
        "{} of {} bricks converted successfully to {} bricks",
        converted.count_success,
        converted.count_success + converted.count_failure,
        converted.write_data.bricks.len(),
    );

    let mut output_file = try_or_die(File::create(output_path), "Failed to create BRS file");

    try_or_die(
        brs::write_save(&mut output_file, &converted.write_data),
        "Failed to write BRS file",
    );
}

struct Args {
    input_paths: Vec<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = std::env::args();
    let program = args.next().unwrap();

    let input_paths: Vec<_> = args.collect();

    if input_paths.is_empty() {
        return Err(program.clone())?;
    }

    Ok(Args { input_paths })
}

fn try_or_die<T, E: std::fmt::Display>(r: Result<T, E>, message_prefix: &str) -> T {
    match r {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: {}", message_prefix, e);
            std::process::exit(1)
        }
    }
}
