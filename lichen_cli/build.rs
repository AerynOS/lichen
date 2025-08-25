use std::{
    env,
    error::Error,
    fs::{self, File},
    io::Write,
    path::Path,
};

const SELECTIONS_DIR: &str = "../selections";

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo::rerun-if-changed={SELECTIONS_DIR}");

    let out_dir = env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("selections.rs");

    let mut selections = File::create(&dest_path)?;

    writeln!(&mut selections, r##"macro_rules! selections {{"##,)?;
    writeln!(&mut selections, r##"() => {{["##,)?;
    for f in fs::read_dir(SELECTIONS_DIR)? {
        let f = f?;

        if !f.file_type()?.is_file() {
            continue;
        }

        if !f.file_name().to_str().unwrap().ends_with(".json") {
            continue;
        }

        writeln!(
            &mut selections,
            r##"Group::from_str(include_str!("../{name}"))?,"##,
            name = f.path().display(),
        )?;
    }

    writeln!(&mut selections, r##"]}};"##,)?;
    writeln!(&mut selections, r##"}}"##,)?;

    Ok(())
}
