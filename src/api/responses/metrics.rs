use opg::OpgModel;
use serde::Serialize;

use crate::models::*;

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("MetricsResponse")]
pub struct MetricsResponse {
    pub gen_utime: u32,
}

impl From<Metrics> for MetricsResponse {
    fn from(r: Metrics) -> Self {
        Self {
            gen_utime: r.gen_utime,
        }
    }
}
