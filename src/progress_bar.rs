use indicatif::{MultiProgress, ProgressDrawTarget, ProgressStyle};

pub struct ProgressBarMult {
    mt: MultiProgress,
    st: ProgressStyle,
}

impl ProgressBarMult {
    pub fn new() -> anyhow::Result<Self> {
        let mt = MultiProgress::new();
        let st = ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )?
        .progress_chars("##-");

        Ok(Self { mt, st })
    }

    pub fn progress_bar(&self) -> ProgressBar {
        let pg = self
            .mt
            .add(indicatif::ProgressBar::hidden().with_style(self.st.clone()));

        ProgressBar(pg)
    }
}

#[derive(Clone)]
pub struct ProgressBar(indicatif::ProgressBar);

impl ProgressBar {
    pub fn setup(&self, len: u64, msg: &'static str) {
        self.0.set_length(len);
        self.0.set_message(msg);
        self.0.set_draw_target(ProgressDrawTarget::stdout());
    }

    pub fn tick(&self) {
        self.0.inc(1);
    }

    pub fn finish(self) {
        self.0.finish();
    }
}
