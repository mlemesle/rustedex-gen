use flume::{Receiver, Sender, unbounded};

use crate::progress_bar::ProgressBar;

pub fn start_workers<Input, Output, Fut, Context>(
    nb_workers: usize,
    pg: &ProgressBar,
    process: fn(Input, Context) -> Fut,
    context: Context,
) -> (Sender<Input>, Receiver<Output>)
where
    Input: Send + 'static,
    Output: Send + 'static,
    Context: Clone + Send + 'static,
    Fut: Future<Output = Output> + Send + 'static,
{
    let (input_sender, input_receiver) = unbounded();
    let (output_sender, output_receiver) = unbounded();

    for _ in 0..nb_workers {
        let inner_input_receiver = input_receiver.clone();
        let inner_output_sender = output_sender.clone();
        let inner_pg = pg.clone();
        let inner_context = context.clone();

        tokio::spawn(async move {
            while let Ok(i) = inner_input_receiver.recv_async().await {
                let res = process(i, inner_context.clone()).await;
                inner_output_sender.send_async(res).await.unwrap();
                inner_pg.tick();
            }
        });
    }

    (input_sender, output_receiver)
}
