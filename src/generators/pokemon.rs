use std::{collections::HashMap, path::PathBuf, sync::Arc};

use futures_util::TryStreamExt;
use rustemon::Follow;
use serde::Serialize;
use tokio::fs;

use crate::{progress_bar::ProgressBar, workers::start_workers};

use super::{FindTrad, GeneratorContext, PokemonSpecie};

#[derive(Serialize)]
struct Context<'a> {
    page_title: &'a str,
    pokemon: PokemonId<'a>,
}

#[derive(Serialize)]
struct PokemonId<'a> {
    sprite: &'a str,
    name: &'a str,
    french_name: &'a str,
    japanese_name: &'a str,
    japanese_romanized: &'a str,
    types: Vec<String>,
    genus: &'a str,
    height: f32,
    weight: f32,
    abilities: Vec<String>,
    egg_groups: Vec<String>,
    egg_hatch_steps: i64,
    effort_points: HashMap<String, i64>,
    base_experience: i64,
    exp_at_100: i64,
    color: &'a str,
    capture_rate: i64,
}

pub(super) async fn generate(
    p: PathBuf,
    gc: GeneratorContext,
    pokemon_species: &[Arc<PokemonSpecie>],
    pg: ProgressBar,
) -> anyhow::Result<()> {
    pg.setup(pokemon_species.len() as u64, "Generating pokemon pages");

    let (ps_input, res_output) =
        start_workers(20, &pg, generate_pokemon_id, PokemonContext { p, gc });

    for ps in pokemon_species {
        ps_input.send_async(Arc::clone(ps)).await?;
    }
    drop(ps_input);

    res_output.into_stream().try_collect::<()>().await?;

    pg.finish();

    Ok(())
}

#[derive(Clone)]
struct PokemonContext {
    p: PathBuf,
    gc: GeneratorContext,
}

async fn generate_pokemon_id(ps: Arc<PokemonSpecie>, pc: PokemonContext) -> anyhow::Result<()> {
    let (Some(name), Some(french_name), Some(japanese_name), Some(japanese_romanized)) =
        ps.s.names
            .iter()
            .fold((None, None, None, None), |mut agg, name| {
                match name.language.name.as_str() {
                    "en" => agg.0 = Some(name.name.as_str()),
                    "fr" => agg.1 = Some(name.name.as_str()),
                    "ja" => agg.2 = Some(name.name.as_str()),
                    "roomaji" => agg.3 = Some(name.name.as_str()),
                    _ => (),
                }
                agg
            })
    else {
        anyhow::bail!("Missing some names for {}", ps.p.id);
    };
    let genus = ps.s.genera.find_trad("genus", ps.p.id)?;

    let mut abilities = Vec::with_capacity(ps.p.abilities.len());
    for a in &ps.p.abilities {
        let aa = a.ability.follow(&pc.gc.rc).await?;
        abilities.push(aa.names.find_trad("ability", ps.p.id)?);
    }

    let mut egg_groups = Vec::with_capacity(ps.s.egg_groups.len());
    for eg in &ps.s.egg_groups {
        let egg = eg.follow(&pc.gc.rc).await?;
        egg_groups.push(egg.names.find_trad("egg group", ps.p.id)?);
    }

    let mut effort_points = HashMap::new();
    for stat in ps.p.stats.iter().filter(|s| s.effort > 0) {
        let s = stat.stat.follow(&pc.gc.rc).await?;
        let k = s.names.find_trad("stat", ps.p.id)?;
        effort_points.insert(k, stat.effort);
    }

    let exp_at_100 =
        ps.s.growth_rate
            .follow(&pc.gc.rc)
            .await?
            .levels
            .iter()
            .find_map(|gr| (gr.level == 100).then_some(gr.experience))
            .ok_or_else(|| anyhow::anyhow!("No experience at lvl 100 for {}", ps.p.id))?;
    let c = ps.s.color.follow(&pc.gc.rc).await?;
    let color = c.names.find_trad("color", ps.p.id)?;
    let sprite =
        ps.p.sprites
            .other
            .official_artwork
            .front_default
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No sprite for {}", ps.p.id))?;

    let pokemon = PokemonId {
        sprite,
        name,
        french_name,
        japanese_name,
        japanese_romanized,
        types: ps.p.types.iter().map(|t| t.type_.name.clone()).collect(),
        genus: genus.as_str(),
        height: ps.p.height as f32 / 10.0,
        weight: ps.p.weight as f32 / 10.0,
        abilities,
        egg_groups,
        egg_hatch_steps: (ps.s.hatch_counter.unwrap_or_default() + 1) * 255,
        effort_points,
        base_experience: ps.p.base_experience.unwrap_or_default(),
        exp_at_100,
        color: color.as_str(),
        capture_rate: ps.s.capture_rate,
    };

    let f = fs::File::create(pc.p.join(format!("{}.html", ps.p.id)))
        .await?
        .into_std()
        .await;

    pc.gc.t.render_to(
        "pokemon.html.tera",
        &tera::Context::from_serialize(Context {
            page_title: name,
            pokemon,
        })?,
        f,
    )?;

    Ok(())
}
