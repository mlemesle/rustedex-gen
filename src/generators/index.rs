use std::{path::Path, sync::Arc};

use serde::Serialize;
use tokio::fs;

use crate::progress_bar::ProgressBar;

use super::{GeneratorContext, PokemonSpecie};

#[derive(Serialize)]
struct Context<'a> {
    page_title: &'a str,
    pokemons: Vec<PokemonCard<'a>>,
}

#[derive(Serialize)]
struct PokemonCard<'a> {
    id: i64,
    pokedex_number: i64,
    name: &'a str,
    sprite: &'a str,
    types: Vec<&'a str>,
}

pub(super) async fn generate(
    p: &Path,
    gc: GeneratorContext,
    pokemon_species: &[Arc<PokemonSpecie>],
    pg: ProgressBar,
) -> anyhow::Result<()> {
    pg.set_length(pokemon_species.len() as u64);
    let pokemon_cards = pokemon_species
        .iter()
        .map(|ps| {
            pg.tick();
            Ok(PokemonCard {
                id: ps.p.id,
                pokedex_number: ps.s.order,
                name: ps
                    .s
                    .names
                    .iter()
                    .find_map(|n| (n.language.name == "en").then_some(&n.name))
                    .ok_or_else(|| anyhow::anyhow!("No name for {}", ps.p.id))?,
                sprite: ps
                    .p
                    .sprites
                    .other
                    .official_artwork
                    .front_default
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("No sprite for {}", ps.p.id))?,
                types: ps.p.types.iter().map(|t| t.type_.name.as_str()).collect(),
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let f = fs::File::create(p.join("index.html"))
        .await?
        .into_std()
        .await;

    gc.t.render_to(
        "index.html.tera",
        &tera::Context::from_serialize(Context {
            page_title: "Index",
            pokemons: pokemon_cards,
        })?,
        f,
    )?;

    pg.finish();

    Ok(())
}
