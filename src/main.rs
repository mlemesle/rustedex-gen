use clap::Parser;
use rustedex_gen::{
    args::Args,
    export,
    generators::{self, GeneratorContext},
};
use rustemon::client::{CACacheManager, RustemonClientBuilder};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    let args = Args::parse();
    export::export(&args.export_path, args.static_path).await?;

    let t = tera::Tera::new("templates/**/*.html.tera")?;
    let rc = RustemonClientBuilder::<CACacheManager>::default()
        // .with_manager(MokaManager::default())
        .try_build()?;

    let gc = GeneratorContext::new(rc, t);

    println!("Generating Rustedex...");
    generators::generate_rustedex(&args.export_path, gc, args.dev).await?;

    if args.dev {
        println!("Web server available at http://localhost:3030/index.html");
        let route = warp::fs::dir(args.export_path);
        warp::serve(route).bind(([0, 0, 0, 0], 3030)).await;
    }

    Ok(())
}
