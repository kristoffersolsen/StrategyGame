//! Background generation thread with channel-based communication.
//!
//! The UI thread sends a [`RunRequest`] on button press. The worker thread
//! calls `generate_world` (blocking) and sends back a [`RunnerResult`].
//! The UI calls [`Runner::try_recv`] each frame — no blocking.

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use rayon::prelude::*;
use worldgen::{BasicParams, WorldData, WorldParams, generate_world};

use crate::sweep;

// ------------- Public types -------------

/// Sent from the UI thread to the worker thread.
pub(crate) enum RunRequest {
    Single {
        params: WorldParams,
        hex_size: u32,
    },
    Sweep {
        base_params: WorldParams,
        param_name: String,
        min: f64,
        max: f64,
        steps: usize,
        extra_seeds: Vec<u64>,
        hex_size: u32,
    },
}

/// Completed world data plus metadata needed by the figure renderers.
pub(crate) struct WorldOutput {
    pub(crate) world: WorldData,
    pub(crate) land_level_count: u8,
    pub(crate) hex_size: u32,
}

/// Sent from the worker thread back to the UI thread.
pub(crate) enum RunnerResult {
    Single(Box<Result<WorldOutput, String>>),
    /// One entry per sweep step; each entry is `(param_value, result)`.
    Sweep(Vec<(f64, Result<WorldOutput, String>)>),
}

/// Owns the worker thread and the communication channels.
pub(crate) struct Runner {
    tx: Sender<RunRequest>,
    rx: Receiver<RunnerResult>,
}

impl Runner {
    /// Spawns the worker thread and returns a connected [`Runner`].
    pub(crate) fn new() -> Self {
        let (req_tx, req_rx) = crossbeam_channel::unbounded::<RunRequest>();
        let (res_tx, res_rx) = crossbeam_channel::unbounded::<RunnerResult>();

        std::thread::Builder::new()
            .name("maptune-worker".to_owned())
            .spawn(move || worker_loop(req_rx, res_tx))
            .expect("failed to spawn maptune worker thread");

        Self {
            tx: req_tx,
            rx: res_rx,
        }
    }

    /// Sends a generation request to the worker thread.
    pub(crate) fn send(&self, req: RunRequest) {
        // The channel is unbounded; the only failure mode is a disconnected
        // receiver, which cannot happen while the worker thread is alive.
        self.tx
            .send(req)
            .expect("worker thread unexpectedly stopped");
    }

    /// Non-blocking poll for a completed result.
    pub(crate) fn try_recv(&self) -> Option<RunnerResult> {
        match self.rx.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                panic!("worker thread disconnected unexpectedly")
            }
        }
    }
}

// ------------- Worker thread -------------

#[allow(clippy::needless_pass_by_value)]
fn worker_loop(rx: Receiver<RunRequest>, tx: Sender<RunnerResult>) {
    for req in rx {
        let result = match req {
            RunRequest::Single { params, hex_size } => {
                RunnerResult::Single(Box::new(run_single(&params, hex_size)))
            }
            RunRequest::Sweep {
                base_params,
                param_name,
                min,
                max,
                steps,
                extra_seeds,
                hex_size,
            } => {
                let result = run_sweep(
                    &base_params,
                    &param_name,
                    min,
                    max,
                    steps,
                    &extra_seeds,
                    hex_size,
                );
                RunnerResult::Sweep(result)
            }
        };
        if tx.send(result).is_err() {
            break;
        }
    }
}

fn run_single(params: &WorldParams, hex_size: u32) -> Result<WorldOutput, String> {
    let land_level_count = params.tuning.tectonics.land_level_count;
    generate_world(params)
        .map(|world| WorldOutput {
            world,
            land_level_count,
            hex_size,
        })
        .map_err(|e| e.to_string())
}

fn run_sweep(
    base_params: &WorldParams,
    param_name: &str,
    min: f64,
    max: f64,
    steps: usize,
    extra_seeds: &[u64],
    hex_size: u32,
) -> Vec<(f64, Result<WorldOutput, String>)> {
    let values = linspace(min, max, steps);

    // Validate the param name before spawning parallel work.
    {
        let mut probe = base_params.tuning.clone();
        if let Err(e) = sweep::set_param(&mut probe, param_name, min) {
            let err_msg = e.to_string();
            return values
                .into_iter()
                .map(|v| (v, Err(err_msg.clone())))
                .collect();
        }
    }

    // For sweep display we use only the base seed plus any extra seeds.
    // Each step renders one image per seed; figure_panel shows the first seed.
    let all_seeds: Vec<u64> = std::iter::once(base_params.seed)
        .chain(extra_seeds.iter().copied())
        .collect();

    values
        .par_iter()
        .map(|&val| {
            // Use only the first seed for the visual output returned per step.
            let seed = all_seeds[0];
            let mut tuning = base_params.tuning.clone();
            let result = sweep::set_param(&mut tuning, param_name, val)
                .and_then(|()| {
                    let mut params = WorldParams {
                        seed,
                        basic: BasicParams {
                            map_size: base_params.basic.map_size,
                            ..BasicParams::default()
                        },
                        ..WorldParams::default()
                    };
                    params.tuning = tuning;
                    let land_level_count = params.tuning.tectonics.land_level_count;
                    generate_world(&params)
                        .map(|world| WorldOutput {
                            world,
                            land_level_count,
                            hex_size,
                        })
                        .map_err(|e| anyhow::anyhow!(e))
                })
                .map_err(|e| e.to_string());
            (val, result)
        })
        .collect()
}

fn linspace(min: f64, max: f64, steps: usize) -> Vec<f64> {
    if steps <= 1 {
        return vec![min];
    }
    #[allow(clippy::cast_precision_loss)]
    (0..steps)
        .map(|i| min + i as f64 * (max - min) / (steps - 1) as f64)
        .collect()
}
