use bls2brs::{bl_save, brs, convert};
use std::{
    fs::File,
    ffi::OsStr,
    io::BufReader,
    path::{Path, PathBuf},
};

fn main() {
    eprintln!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    eprintln!();

    if let Err(e) = run() {
        eprintln!("{}", e);
        eprintln!();
        wexit::prompt_enter_to_exit(1);
    }

    eprintln!();
    wexit::prompt_enter_to_exit(0);
}

fn run() -> Result<(), String> {
    let args = parse_args()
        .map_err(|_| String::from("Error: No bls files given. Drag them onto this program's executable file. (Not this window! This is just an error message, not the program itself.)"))?;

    for (i, input_path) in args.input_paths.iter().enumerate() {
        if i > 0 {
            println!();
        }

        let input_path = PathBuf::from(input_path);

        println!("Converting {}", input_path.display());

        if input_path.extension() != Some(OsStr::new("bls")) {
            println!("Extension is not .bls, skipping");
            continue;
        }

        let mut output_path = input_path.clone();

        output_path.set_extension("brs");

        convert_one(&input_path, &output_path)
            .map_err(|e| format!("Error converting {}: {}", input_path.display(), e))?;
    }

    Ok(())
}

fn convert_one(input_path: impl AsRef<Path>, output_path: impl AsRef<Path>) -> Result<(), String> {
    let input_path = input_path.as_ref();
    let output_path = output_path.as_ref();

    let input_file = errmsg(File::open(input_path), "Failed to open bls file")?;
    let input_file = BufReader::new(input_file);
    let input_reader = errmsg(bl_save::Reader::new(input_file), "Failed to read bls file")?;

    let mut converted = errmsg(convert(input_reader), "Failed to convert bls file")?;

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
            let ui_name = if ui_name != ui_name.trim() {
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

    let mut output_file = errmsg(File::create(output_path), "Failed to create BRS file")?;

    errmsg(
        brs::write_save(&mut output_file, &converted.write_data),
        "Failed to write BRS file",
    )?;

    Ok(())
}

struct Args {
    input_paths: Vec<String>,
}

fn parse_args() -> Result<Args, ()> {
    let mut args = std::env::args();
    args.next().unwrap();

    let input_paths: Vec<_> = args.collect();

    if input_paths.is_empty() {
        return Err(())?;
    }

    Ok(Args { input_paths })
}

fn errmsg<T, E: std::fmt::Display>(r: Result<T, E>, message_prefix: &str) -> Result<T, String> {
    r.map_err(|e| format!("{}: {}", message_prefix, e))
}
