use std::{path::Path, sync::Arc};

use rustemon::{
    Follow,
    client::RustemonClient,
    model::{pokemon::Pokemon, resource::NamedApiResource},
};
use serde::Serialize;
use tokio::fs;

use super::GeneratorContext;

#[derive(Serialize)]
struct Context {
    page_title: &'static str,
    pokemons: Vec<PokemonCard>,
}

#[derive(Serialize)]
struct PokemonCard {
    id: i64,
    pokedex_number: i64,
    name: String,
    sprite: String,
    types: Vec<String>,
}

pub(super) async fn generate(p: &Path, gc: GeneratorContext) -> anyhow::Result<()> {
    let pokemon_entries = rustemon::pokemon::pokemon::get_all_entries(&gc.rc).await?;
    println!("About to export {} pokemons", pokemon_entries.len());

    let mut pokemon_cards = Vec::new();
    let (npt_sndr, npt_rcvr) = flume::unbounded::<NamedApiResource<Pokemon>>();
    let (tpt_sndr, tpt_rcvr) = flume::unbounded::<anyhow::Result<PokemonCard>>();

    for _ in 0..20 {
        let rc = Arc::clone(&gc.rc);
        let inner_npt_rcvr = npt_rcvr.clone();
        let inner_tpt_sndr = tpt_sndr.clone();

        tokio::spawn(async move {
            while let Ok(pe) = inner_npt_rcvr.recv_async().await {
                match generate_pokemon_card(pe, &rc).await {
                    Ok(Some(pc)) => inner_tpt_sndr.send_async(Ok(pc)).await.unwrap(),
                    Ok(None) => (),
                    Err(err) => inner_tpt_sndr.send_async(Err(err)).await.unwrap(),
                }
            }
        });
    }

    for pokemon_entry in pokemon_entries {
        npt_sndr.send_async(pokemon_entry).await?;
    }
    drop(npt_sndr);
    drop(tpt_sndr);

    while let Ok(pc) = tpt_rcvr.recv_async().await {
        pokemon_cards.push(pc?);
    }
    pokemon_cards.sort_by_key(|pc| pc.pokedex_number);

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

    Ok(())
}

async fn generate_pokemon_card(
    pokemon_entry: NamedApiResource<Pokemon>,
    rc: &RustemonClient,
) -> anyhow::Result<Option<PokemonCard>> {
    let pokemon = pokemon_entry.follow(rc).await?;
    if pokemon.is_default {
        let specie = pokemon.species.follow(rc).await?;
        let name = specie
            .names
            .into_iter()
            .find(|n| n.language.name == "en")
            .map(|n| n.name);
        Ok(Some(PokemonCard {
            id: pokemon.id,
            pokedex_number: specie.order,
            name: name.ok_or_else(|| anyhow::anyhow!("No name for {}", pokemon.id))?,
            sprite: pokemon
                .sprites
                .other
                .official_artwork
                .front_default
                .ok_or_else(|| anyhow::anyhow!("No sprite for {}", pokemon.id))?,
            types: pokemon.types.into_iter().map(|t| t.type_.name).collect(),
        }))
    } else {
        Ok(None)
    }
}
