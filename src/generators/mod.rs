use std::{path::Path, sync::Arc};

use futures_util::stream;
use futures_util::TryStreamExt;
use rustemon::{
    Follow,
    client::RustemonClient,
    model::{
        pokemon::{Genus, Pokemon, PokemonSpecies},
        resource::{Name, NamedApiResource},
    },
};
use tera::Tera;

use crate::progress_bar::{ProgressBar, ProgressBarMult};

mod index;
mod pokemon;

// #[derive(Clone)]
pub struct GeneratorContext {
    rc: RustemonClient,
    t: Tera,
}

impl GeneratorContext {
    pub fn new(rc: RustemonClient, t: Tera) -> Self {
        Self { rc, t }
    }
}

pub struct PokemonSpecie {
    p: Pokemon,
    s: PokemonSpecies,
}

pub async fn generate_rustedex(
    rustedex_path: &Path,
    gc: &GeneratorContext,
    dev: bool,
) -> anyhow::Result<()> {
    let pgm = ProgressBarMult::new()?;

    let pokemon_species =
        get_all_pokemon_species(&gc, dev, pgm.progress_bar("Fetching Pokemons")).await?;

    tokio::try_join!(
        index::generate(
            rustedex_path,
            gc,
            &pokemon_species,
            pgm.progress_bar("Generating cards for index"),
        ),
        pokemon::generate(
            rustedex_path.join("pokemon"),
            gc,
            pokemon_species.as_slice(),
            pgm.progress_bar("Generating pokemon pages"),
        )
    )?;

    Ok(())
}

async fn get_all_pokemon_species(
    gc: &GeneratorContext,
    dev: bool,
    pg: ProgressBar,
) -> anyhow::Result<Vec<Arc<PokemonSpecie>>> {
    let mut pokemon_entries = rustemon::pokemon::pokemon::get_all_entries(&gc.rc).await?;
    if dev {
        pokemon_entries.truncate(1000);
    }

    pg.set_length(pokemon_entries.len() as u64);

    let mut pokemon_species = stream::FuturesUnordered::from_iter(
        pokemon_entries
            .into_iter()
            .map(|x| generate_pokemon_specie(x, gc)),
    )
    .try_filter_map(|x| async {
        pg.tick();
        if let Some(e) = x {
            Ok(Some(Arc::new(e)))
        } else {
            Ok(None)
        }
    })
    .try_collect::<Vec<Arc<PokemonSpecie>>>()
    .await?;

    pokemon_species.sort_by_key(|ps| ps.p.order);

    pg.finish();

    Ok(pokemon_species)
}

async fn generate_pokemon_specie(
    pokemon_entry: NamedApiResource<Pokemon>,
    gc: &GeneratorContext,
) -> anyhow::Result<Option<PokemonSpecie>> {
    let pokemon = pokemon_entry.follow(&gc.rc).await?;
    if pokemon.is_default {
        let specie = pokemon.species.follow(&gc.rc).await?;
        Ok(Some(PokemonSpecie {
            p: pokemon,
            s: specie,
        }))
    } else {
        Ok(None)
    }
}

trait FindTrad
where
    Self: IntoIterator,
{
    fn find_trad(&self, type_: &str, id: i64) -> anyhow::Result<String>;
}

impl FindTrad for Vec<Name> {
    fn find_trad(&self, type_: &str, id: i64) -> anyhow::Result<String> {
        self.iter()
            .find_map(|n| (n.language.name == "en").then_some(n.name.clone()))
            .ok_or_else(|| anyhow::anyhow!("Can't find trad for {type_} for {id}"))
    }
}

impl FindTrad for Vec<Genus> {
    fn find_trad(&self, type_: &str, id: i64) -> anyhow::Result<String> {
        self.iter()
            .find_map(|n| (n.language.name == "en").then_some(n.genus.clone()))
            .ok_or_else(|| anyhow::anyhow!("Can't find trad for {type_} for {id}"))
    }
}
