use comfy_table::{Cell, Table};
use serde_json::{Map, Value};

use super::Output;
use crate::error::AppError;

/// 将统一输出渲染为适合交互终端阅读的表格或文本。
pub fn render(output: Output) -> Result<(), AppError> {
    match output {
        Output::Text(text) => println!("{text}"),
        Output::Json(Value::Object(object)) => render_objects(&[object]),
        Output::Json(Value::Array(values)) => {
            let objects = values
                .into_iter()
                .map(|value| match value {
                    Value::Object(object) => Ok(object),
                    _ => Err(AppError::Serialization(
                        "table output requires JSON objects".to_string(),
                    )),
                })
                .collect::<Result<Vec<_>, _>>()?;
            render_objects(&objects);
        }
        Output::Json(value) => println!("{}", display_value(&value)),
        Output::Empty => {}
    }
    Ok(())
}

fn render_objects(objects: &[Map<String, Value>]) {
    if objects.is_empty() {
        return;
    }

    let headers = objects[0].keys().cloned().collect::<Vec<_>>();
    let mut table = Table::new();
    table.set_header(headers.iter().map(Cell::new));

    for object in objects {
        table.add_row(
            headers
                .iter()
                .map(|header| Cell::new(object.get(header).map(display_value).unwrap_or_default())),
        );
    }

    println!("{table}");
}

fn display_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        _ => value.to_string(),
    }
}
