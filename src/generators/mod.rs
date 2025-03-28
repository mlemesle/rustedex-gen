use std::{path::Path, sync::Arc};

use rustemon::client::RustemonClient;
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

pub async fn generate_rustedex(rustedex_path: &Path, gc: GeneratorContext) -> anyhow::Result<()> {
    index::generate(rustedex_path, gc).await?;

    Ok(())
}
