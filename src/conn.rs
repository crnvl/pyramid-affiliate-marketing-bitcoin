use std::sync::Arc;

use crate::{Config, Pixel};
use anyhow::{anyhow, Result};
use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
    sync::{mpsc, oneshot},
    task::JoinSet,
};

async fn connection(
    server: String,
    conn_id: usize,
    num_conns: usize,
) -> Result<mpsc::UnboundedSender<(Arc<Vec<Pixel>>, oneshot::Sender<usize>)>> {
    let (tx, mut rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        let mut tcp_tx = TcpStream::connect(&server).await.unwrap().into_split().1;

        loop {
            let (buffer, oneshot_tx): (Arc<Vec<Pixel>>, oneshot::Sender<usize>) =
                rx.recv().await.unwrap();
            let mut errors = 0;

            let mut num_px = buffer.len() / num_conns;

            if buffer.len() % num_conns > conn_id {
                num_px += 1;
            }

            for i in 0..num_px {
                let idx = (i * num_conns) + conn_id;

                // if idx % 128 == 0 {
                //     tokio::task::yield_now().await;
                // }

                let px = &buffer[idx];
                let command = format!(
                    "PX {x} {y} {r:02x}{g:02x}{b:02x}{a:02x}\n",
                    x = px.x,
                    y = px.y,
                    r = px.value[0],
                    g = px.value[1],
                    b = px.value[2],
                    a = px.value[3]
                );

                loop {
                    match tcp_tx.write(command.as_bytes()).await {
                        Err(_e) => {
                            // println!("Error: {e}");
                            errors += 1;
                            tcp_tx = TcpStream::connect(&server).await.unwrap().into_split().1;
                        }
                        Ok(_) => break,
                    };
                }
            }
            oneshot_tx.send(errors).unwrap();
        }
    });

    Ok(tx)
}

pub struct ConnectionBundle {
    tx: mpsc::UnboundedSender<Job>,
}

pub enum Job {
    UpdateBuffer {
        buffer: Vec<Pixel>,
        restore: Option<Vec<Pixel>>,
    },
}

#[derive(Default, Debug)]
pub struct Stats {
    pub errors: usize,
}

impl ConnectionBundle {
    pub async fn new(config: Config, stats_tx: mpsc::UnboundedSender<Stats>) -> Result<Self> {
        let (mpsc_tx, mut mpsc_rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            let mut buffer: Arc<Vec<Pixel>> = Arc::new(Vec::new());
            let mut restore: Option<Vec<Pixel>> = None;

            let mut connections = Vec::with_capacity(config.threads);
            for i in 0..config.threads {
                connections.push(
                    connection(config.server.clone(), i, config.threads)
                        .await
                        .unwrap(),
                );
            }

            loop {
                if !mpsc_rx.is_empty() || buffer.len() == 0 {
                    match mpsc_rx.recv().await.unwrap() {
                        Job::UpdateBuffer {
                            buffer: new_buffer,
                            restore: new_restore,
                        } => {
                            if let Some(restore) = restore {
                                draw(&mut connections, &Arc::new(restore), stats_tx.clone())
                                    .await
                                    .unwrap();
                            }
                            restore = new_restore;
                            // TODO: fetch restore pixels

                            buffer = Arc::new(new_buffer);
                        }
                    }
                }

                draw(&mut connections, &buffer, stats_tx.clone())
                    .await
                    .unwrap();
            }
        });

        Ok(Self { tx: mpsc_tx })
    }

    pub fn update_buffer(&self, buffer: Vec<Pixel>, restore: Option<Vec<Pixel>>) -> Result<()> {
        self.tx.send(Job::UpdateBuffer { buffer, restore })?;
        Ok(())
    }
}

async fn draw(
    connections: &mut Vec<mpsc::UnboundedSender<(Arc<Vec<Pixel>>, oneshot::Sender<usize>)>>,
    buffer: &Arc<Vec<Pixel>>,
    stats_tx: mpsc::UnboundedSender<Stats>,
) -> Result<()> {
    let mut set = JoinSet::new();
    for conn in connections.iter() {
        let (tx, rx) = oneshot::channel();
        conn.send((buffer.clone(), tx))?;
        set.spawn(rx);
    }

    let mut stats = Stats::default();
    while let Some(res) = set.join_next().await {
        stats.errors += res??;
    }
    stats_tx.send(stats).map_err(|e| anyhow!("{e}"))
}
