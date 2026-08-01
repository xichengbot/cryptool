use super::Output;
use crate::error::AppError;

/// 将统一输出渲染为机器可读 JSON。
///
/// 文本与空输出也封装为 JSON，保持 `--output json` 的输出契约一致。
pub fn render(output: Output) -> Result<(), AppError> {
    match output {
        Output::Text(text) => println!("{}", serde_json::json!({ "message": text })),
        Output::Json(value) => {
            let text = serde_json::to_string_pretty(&value)
                .map_err(|err| AppError::Serialization(err.to_string()))?;
            println!("{text}");
        }
        Output::Empty => println!("{}", serde_json::json!({})),
    }
    Ok(())
}
