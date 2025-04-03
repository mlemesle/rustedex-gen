use std::{path::Path, sync::Arc};

use futures_util::TryStreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use rustemon::{
    Follow,
    client::RustemonClient,
    model::{
        pokemon::{Genus, Pokemon, PokemonSpecies},
        resource::{Name, NamedApiResource},
    },
};
use tera::Tera;

mod index;
mod pokemon;

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

pub async fn generate_rustedex(
    rustedex_path: &Path,
    gc: GeneratorContext,
    dev: bool,
) -> anyhow::Result<()> {
    let mt = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )?
    .progress_chars("##-");

    let pokemon_species = get_all_pokemon_species(
        &gc,
        dev,
        mt.add(ProgressBar::hidden().with_style(sty.clone())),
    )
    .await?;

    index::generate(
        rustedex_path,
        gc.clone(),
        &pokemon_species,
        mt.add(ProgressBar::hidden().with_style(sty.clone())),
    )
    .await?;

    pokemon::generate(
        rustedex_path.join("pokemon"),
        gc,
        pokemon_species.as_slice(),
        mt.add(ProgressBar::hidden().with_style(sty.clone())),
    )
    .await?;

    Ok(())
}

async fn get_all_pokemon_species(
    gc: &GeneratorContext,
    dev: bool,
    pg: ProgressBar,
) -> anyhow::Result<Vec<Arc<PokemonSpecie>>> {
    let mut pokemon_entries = rustemon::pokemon::pokemon::get_all_entries(&gc.rc).await?;
    if dev {
        pokemon_entries.truncate(500);
    }

    pg.set_length(pokemon_entries.len() as u64);
    pg.set_message("Fetching Pokemons");
    pg.set_draw_target(ProgressDrawTarget::stdout());

    let (npt_sndr, npt_rcvr) = flume::unbounded::<NamedApiResource<Pokemon>>();
    let (tpt_sndr, tpt_rcvr) = flume::unbounded::<anyhow::Result<PokemonSpecie>>();

    let generate_pokemon_specie = async |pokemon_entry: NamedApiResource<Pokemon>,
                                         rc: &RustemonClient|
           -> anyhow::Result<Option<PokemonSpecie>> {
        let pokemon = pokemon_entry.follow(rc).await?;
        if pokemon.is_default {
            let specie = pokemon.species.follow(rc).await?;
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
        let inner_pg = pg.clone();

        tokio::spawn(async move {
            while let Ok(pe) = inner_npt_rcvr.recv_async().await {
                match generate_pokemon_specie(pe, &rc).await {
                    Ok(Some(pc)) => inner_tpt_sndr.send_async(Ok(pc)).await.unwrap(),
                    Ok(None) => (),
                    Err(err) => inner_tpt_sndr.send_async(Err(err)).await.unwrap(),
                }
                inner_pg.inc(1);
            }
        });
    }

    for pokemon_entry in pokemon_entries {
        npt_sndr.send_async(pokemon_entry).await?;
    }
    drop(npt_sndr);
    drop(tpt_sndr);

    let mut pokemon_species: Vec<Arc<PokemonSpecie>> = tpt_rcvr
        .into_stream()
        .map_ok(Arc::new)
        .try_collect()
        .await?;
    pokemon_species.sort_by_key(|ps| ps.p.order);

    pg.finish();

    Ok(pokemon_species)
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
