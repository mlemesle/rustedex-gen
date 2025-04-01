use std::{path::Path, sync::Arc};

use futures_util::TryStreamExt;
use rustemon::{
    Follow,
    client::RustemonClient,
    model::{
        pokemon::{Pokemon, PokemonSpecies},
        resource::NamedApiResource,
    },
};
use tera::Tera;

mod index;

#[derive(Clone)]
pub struct GeneratorContext {
    rc: Arc<RustemonClient>,
    t: Arc<Tera>,
}

impl GeneratorContext {
    pub fn new(rc: RustemonClient, t: Tera) -> Self {
        Self {
            rc: Arc::new(rc),
            t: Arc::new(t),
        }
    }
}

pub struct PokemonSpecie {
    p: Pokemon,
    s: PokemonSpecies,
}

pub async fn generate_rustedex(rustedex_path: &Path, gc: GeneratorContext) -> anyhow::Result<()> {
    let pokemon_species = get_all_pokemon_species(&gc).await?;
    index::generate(rustedex_path, gc, &pokemon_species).await?;

    Ok(())
}

async fn get_all_pokemon_species(gc: &GeneratorContext) -> anyhow::Result<Vec<PokemonSpecie>> {
    let pokemon_entries = rustemon::pokemon::pokemon::get_all_entries(&gc.rc).await?;
    let (npt_sndr, npt_rcvr) = flume::unbounded::<NamedApiResource<Pokemon>>();
    let (tpt_sndr, tpt_rcvr) = flume::unbounded::<anyhow::Result<PokemonSpecie>>();

    let generate_pokemon_specie = async |pokemon_entry: NamedApiResource<Pokemon>,
                                         rc: &RustemonClient|
           -> anyhow::Result<Option<PokemonSpecie>> {
        let pokemon = pokemon_entry.follow(rc).await?;
        let specie = pokemon.species.follow(rc).await?;
        if pokemon.is_default {
            Ok(Some(PokemonSpecie {
                p: pokemon,
                s: specie,
            }))
        } else {
            Ok(None)
        }
    };

    for _ in 0..20 {
        let rc = Arc::clone(&gc.rc);
        let inner_npt_rcvr = npt_rcvr.clone();
        let inner_tpt_sndr = tpt_sndr.clone();

        tokio::spawn(async move {
            while let Ok(pe) = inner_npt_rcvr.recv_async().await {
                match generate_pokemon_specie(pe, &rc).await {
                    Ok(Some(pc)) => inner_tpt_sndr.send_async(Ok(pc)).await.unwrap(),
                    Ok(None) => (),
                    Err(err) => inner_tpt_sndr.send_async(Err(err)).await.unwrap(),
                }
            }
        });
    }

    for pokemon_entry in pokemon_entries.into_iter().take(10) {
        npt_sndr.send_async(pokemon_entry).await?;
    }
    drop(npt_sndr);
    drop(tpt_sndr);

    let mut pokemon_species: Vec<PokemonSpecie> = tpt_rcvr.into_stream().try_collect().await?;
    pokemon_species.sort_by_key(|ps| ps.p.order);

    Ok(pokemon_species)
}
