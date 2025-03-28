use std::path::PathBuf;

use tokio::fs;

pub async fn export(export_path: &PathBuf, static_path: PathBuf) -> anyhow::Result<()> {
    if export_path.exists() {
        if !export_path.is_dir() {
            return Err(anyhow::anyhow!(
                "Can't export to {export_path:?} which isn't a directory."
            ));
        }
        if prompt_user(export_path)? {
            fs::remove_dir_all(&export_path).await?;
        }
    }

    fs::create_dir(export_path).await?;

    let static_export_path = export_path.join("static");
    fs::create_dir(&static_export_path).await?;
    fs::create_dir(static_export_path.join("fonts")).await?;
    fs::copy(
        static_path.join("pokedex.css"),
        static_export_path.join("pokedex.css"),
    )
    .await?;
    fs::copy(
        static_path.join("fonts/pokemonsolid.ttf"),
        static_export_path.join("fonts/pokemonsolid.ttf"),
    )
    .await?;

    Ok(())
}

fn prompt_user(p: &PathBuf) -> anyhow::Result<bool> {
    let ans = inquire::Confirm::new(&format!(
        "Do you want to delete directory {p:?} to regenerate it ?"
    ))
    .with_default(false)
    .with_help_message("Please check the path, as this operation is irreversible.")
    .prompt()?;

    Ok(ans)
}
