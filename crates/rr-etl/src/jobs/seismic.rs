//! Job: seismic (in progress).

use super::{Ctx, JobOutput};
use crate::{Result, data_err};

/// Run the job.
pub fn run(_ctx: &Ctx) -> Result<JobOutput> {
    Err(data_err("the seismic job is not implemented yet"))
}
