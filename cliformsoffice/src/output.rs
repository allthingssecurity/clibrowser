use crate::error::OfficeError;
use serde::Serialize;
use serde_json::Value;

pub struct OutputConfig {
    pub json: bool,
    pub quiet: bool,
}

impl OutputConfig {
    pub fn print_json<T: Serialize>(&self, value: &T) {
        let mut obj = serde_json::to_value(value).unwrap_or(Value::Null);
        if let Value::Object(ref mut map) = obj {
            let ok = Value::Bool(true);
            let mut new_map = serde_json::Map::new();
            new_map.insert("ok".to_string(), ok);
            for (k, v) in map.iter() {
                new_map.insert(k.clone(), v.clone());
            }
            obj = Value::Object(new_map);
        }
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
    }

    pub fn print_human(&self, text: &str) {
        if !self.quiet {
            println!("{}", text);
        }
    }

    pub fn print_error(&self, err: &OfficeError) {
        if self.json {
            let obj = serde_json::json!({
                "ok": false,
                "error": err.error_code(),
                "message": err.to_string(),
            });
            eprintln!("{}", serde_json::to_string_pretty(&obj).unwrap());
        } else {
            eprintln!("error: {}", err);
        }
    }

    pub fn print_anyhow_error(&self, err: &anyhow::Error) {
        if self.json {
            let obj = serde_json::json!({
                "ok": false,
                "error": "error",
                "message": err.to_string(),
            });
            eprintln!("{}", serde_json::to_string_pretty(&obj).unwrap());
        } else {
            eprintln!("error: {}", err);
        }
    }
}
