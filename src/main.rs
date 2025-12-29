mod conn;
mod edges;
mod filter;

use anyhow::{anyhow, Result};
use clap::Parser;
use ffmpeg_sidecar::{command::FfmpegCommand, event::FfmpegEvent};
use image::Rgba;
use std::{collections::HashMap, time::Duration};
use tokio::{
    io::{stdout, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
    time::{interval, Instant},
};

use crate::{
    conn::{ConnectionBundle, Stats},
    edges::{Edge, Edges},
    filter::{Blend, Bounce, Filter, Glitch, Rainbow},
};

#[derive(Debug, Clone, Copy)]
pub struct Pixel {
    x: u32,
    y: u32,
    value: Rgba<u8>,
    edges: Edges,
}

const RESTORE_DEBUG_COLOR: [u8; 4] = [0, 0, 0, 0xff];

async fn start_display(threads: usize) -> Result<mpsc::UnboundedSender<Stats>> {
    let (tx, mut rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        let mut errors = 0;

        loop {
            let stats: Stats = rx.recv().await.unwrap();
            errors += stats.errors;

            stdout()
                .write(format!("\rThreads: {threads}  |  Errors: {errors}").as_bytes())
                .await
                .unwrap();
            stdout().flush().await.unwrap();
        }
    });

    Ok(tx)
}

async fn fetch_canvas_size(server: &str) -> Result<(u32, u32)> {
    let (mut rx, mut tx) = TcpStream::connect(&server).await?.into_split();

    let mut answer = String::new();
    tx.write(b"SIZE\n").await?;
    loop {
        let mut byte = [0];
        if rx.read(&mut byte).await? != 1 {
            return Err(anyhow!("Server returned early EOF on SIZE command"));
        }

        if !byte[0].is_ascii() {
            return Err(anyhow!("Server returned non ascii answer on SIZE command"));
        }

        if byte[0] == b'\n' {
            break;
        }

        answer.push(byte[0] as char);
    }

    let parts: Vec<&str> = answer.split(' ').skip(1).collect();
    if parts.len() != 2 {
        return Err(anyhow!("Server returned invalid answer for SIZE command"));
    }

    let x = parts[0].parse()?;
    let y = parts[1].parse()?;

    Ok((x, y))
}

fn calc_edges(buffer: &mut Vec<Pixel>) -> Result<()> {
    let mut area = Area {
        origin_x: u32::MAX,
        origin_y: u32::MAX,
        size_x: 0,
        size_y: 0,
    };
    for px in buffer.iter() {
        if px.x > area.size_x {
            area.size_x = px.x;
        }
        if px.y > area.size_y {
            area.size_y = px.y;
        }
    }

    let mut temp: Vec<Vec<Option<usize>>> =
        vec![vec![None; area.size_y as usize + 1]; area.size_x as usize + 1];
    for (i, px) in buffer.iter().enumerate() {
        temp[px.x as usize][px.y as usize] = Some(i);
    }

    let mut edges = Vec::with_capacity(4);
    for (x, line) in temp.iter().enumerate() {
        for (y, i) in line.iter().enumerate() {
            if let Some(i) = i {
                edges.clear();

                if y == 0 || temp[x][y - 1].is_none() {
                    edges.push(Edge::Top);
                }

                if x == area.size_x as usize || temp[x + 1][y].is_none() {
                    edges.push(Edge::Right);
                }

                if y == area.size_y as usize || temp[x][y + 1].is_none() {
                    edges.push(Edge::Bottom);
                }

                if x == 0 || temp[x - 1][y].is_none() {
                    edges.push(Edge::Left);
                }

                buffer[*i].edges = Edges::new(edges.as_slice());
            }
        }
    }

    Ok(())
}

#[derive(Parser)]
struct Args {
    /// The servers address
    #[arg(
        short = 's',
        long,
        value_name = "ADDRESS",
        default_value = "wall.c3pixelflut.de"
    )]
    server: String,

    /// The servers port
    #[arg(short = 'p', long, default_value_t = 1337)]
    port: u16,

    /// The amount of threads (concurrent connections) that should be used
    #[arg(short = 't', long, value_name = "NUM", default_value_t = 12)]
    threads: usize,

    /// The file to load the base image / video from
    #[arg(short = 'f', long)]
    file: String,

    /// The targeted animation and video fps
    #[arg(long, value_name = "FPS")]
    target_fps: Option<u32>,

    /// Restores pixels after they are not occupied anymore
    #[arg(short = 'r', long)]
    restore: bool,

    /// Offset of the image on the x axis
    #[arg(short = 'x', value_name = "PX")]
    offset_x: Option<u32>,

    /// Offset of the image on the y axis
    #[arg(short = 'y', value_name = "PX")]
    offset_y: Option<u32>,

    /// Adds the rainbow filter
    #[arg(long, value_name = "BLEND")]
    rainbow: Option<String>,

    /// Adds the bounce filter
    #[arg(long, value_name = "SPEED")]
    bounce: Option<i8>,

    /// Colorizes the image with <rrggbbaa>
    #[arg(long, value_name = "RRGGBBAA")]
    blend: Option<String>,

    /// Makes the image glitch by <factor>
    #[arg(long, value_name = "FACTOR")]
    glitch: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Area {
    origin_x: u32,
    origin_y: u32,
    size_x: u32,
    size_y: u32,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub server: String,
    pub threads: usize,
    pub restore: bool,
    pub canvas_size: (u32, u32),
    pub image_area: Area,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let server = format!("{server}:{port}", server = args.server, port = args.port);
    let canvas_size = fetch_canvas_size(&server).await?;

    let display_tx = start_display(args.threads).await?;

    let mut decoder = FfmpegCommand::new()
        .hide_banner()
        .input(&args.file)
        .args("-f rawvideo -pix_fmt rgba -".split(' '))
        .spawn()?;

    let (mut width, mut height) = (0, 0);

    let mut frames: Vec<(f32, Vec<Pixel>, Vec<Pixel>, HashMap<(u32, u32), usize>)> = Vec::new();
    for event in decoder.iter()? {
        match event {
            FfmpegEvent::OutputFrame(frame) => {
                print!("\rLoading frame {}...", frame.frame_num);
                stdout().flush().await?;

                width = frame.width;
                height = frame.height;

                let mut frame_vec = Vec::with_capacity((width * height) as usize);
                let mut frame_lookup = HashMap::with_capacity((width * height) as usize);

                for (i, pixel) in frame.data.chunks(4).enumerate() {
                    let x = (i as u32 % frame.width) + args.offset_x.unwrap_or_default();
                    let y = (i as u32 / frame.width) + args.offset_y.unwrap_or_default();

                    if pixel[3] != 0 {
                        frame_vec.push(Pixel {
                            x,
                            y,
                            value: Rgba::from([pixel[0], pixel[1], pixel[2], pixel[3]]),
                            edges: Edges::default(),
                        });
                        frame_lookup.insert((x, y), frame_vec.len() - 1);
                    }
                }

                calc_edges(&mut frame_vec)?;
                frames.push((frame.timestamp, frame_vec, Vec::new(), frame_lookup));
            }
            // FfmpegEvent::Log(_level, log) => println!("[ffmpeg] {log}"),
            _ => (),
        }
    }

    let num_frames = frames.len();
    println!("\rLoading {num_frames} frames... success");

    // Iterate over
    if args.restore {
        for i in 0..num_frames {
            let (frame, next) = if i == num_frames - 1 {
                let split = frames.split_at_mut(1);
                let len = split.1.len();
                (&mut split.1[len - 1], &mut split.0[0])
            } else {
                let split = frames.split_at_mut(i + 1);
                (&mut split.0[i], &mut split.1[0])
            };

            let buf = &frame.1;

            for px in buf.iter() {
                if px.value[3] == 0 || next.3.contains_key(&(px.x, px.y)) {
                    continue;
                }

                frame.2.push(Pixel {
                    x: px.x,
                    y: px.y,
                    value: Rgba::from(RESTORE_DEBUG_COLOR),
                    edges: Edges::default(),
                });
            }
        }
    }

    let frames: Vec<(f32, Vec<Pixel>, Vec<Pixel>)> = frames
        .into_iter()
        .map(|(time, data, restore, _lookup)| (time, data, restore))
        .collect();

    println!("Preprocessed {} frames successfully", num_frames);

    let config = Config {
        server,
        threads: args.threads,
        restore: args.restore,
        canvas_size,
        image_area: Area {
            origin_x: args.offset_x.unwrap_or_default(),
            origin_y: args.offset_y.unwrap_or_default(),
            size_x: width,
            size_y: height,
        },
    };

    let mut filters: Vec<Box<dyn Filter>> = Vec::new();

    if let Some(alpha) = args.rainbow {
        filters.push(Box::new(Rainbow::new(u8::from_str_radix(&alpha, 16)?, 10)));
    }

    if let Some(speed) = args.bounce {
        filters.push(Box::new(Bounce::new(&config, speed)));
    }

    if let Some(color) = args.blend {
        let mut buf = [0; 4];
        for i in 0..4 {
            let idx = i * 2;
            buf[i] = u8::from_str_radix(&color[idx..(idx + 2)], 16)?;
        }
        filters.push(Box::new(Blend::new(image::Rgba::from(buf))));
    }

    if let Some(factor) = args.glitch {
        filters.push(Box::new(Glitch::new(&config, factor as i32)));
    }

    let connection = ConnectionBundle::new(config.clone(), display_tx.clone()).await?;

    println!(
        "Starting to flood {width}x{height} source on {}x{} canvas [{}]",
        canvas_size.0, canvas_size.1, config.server
    );

    let mut interval = if let Some(fps) = args.target_fps {
        Some(interval(Duration::from_secs_f64(1.0 / fps as f64)))
    } else {
        None
    };

    loop {
        let timer = Instant::now();

        for (t, frame, res) in frames.iter() {
            if let Some(interval) = &mut interval {
                interval.tick().await;
            } else {
                let duration = Duration::from_secs_f32(*t).saturating_sub(timer.elapsed());
                tokio::time::sleep(duration).await;
            }

            let mut buffer = frame.clone();
            let mut restore = if args.restore {
                Some(res.clone())
            } else {
                None
            };
            for filter in filters.iter_mut() {
                filter.transform_buffer(&mut buffer, &mut restore);
            }

            connection.update_buffer(buffer, restore)?;
        }
    }
}
