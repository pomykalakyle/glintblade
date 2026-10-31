//! The single default lighting pass, applied after authored regional lights and before compose.

use super::Frame;
use crate::world::{Region, Zone};

/// Apply cached static lighting and live caster shadows without changing authored materials.
pub fn apply(zone: &Zone, region: &dyn Region, f: &mut Frame, t: u64) {
    super::lighting::apply(zone, region, f, t);
}
