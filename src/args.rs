use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct Args {
    #[arg(short, long, env)]
    pub export_path: PathBuf,
    #[arg(short, long, env)]
    pub templates_glob: String,
    #[arg(short, long, env)]
    pub static_path: PathBuf,
    #[arg(long, env)]
    pub dev: bool,
}
