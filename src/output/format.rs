use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// 面向终端的输出格式。
///
/// 同时派生 `ValueEnum`，因此可直接作为 clap 的 `--output` 参数值。
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, Eq, PartialEq, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
}
