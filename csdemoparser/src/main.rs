use std::{env, error, fs::File};

fn main() -> Result<(), Box<dyn error::Error>> {
    let mut args = env::args();
    args.next();
    let dem_path = args.next().ok_or("need dem file path")?;
    let mut demo_file = File::open(dem_path)?;
    let demoinfo = csdemoparser::parse(&mut demo_file)?;
    serde_json::to_writer(std::io::stdout(), &demoinfo)?;
    Ok(())
}
