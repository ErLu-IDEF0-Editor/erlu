wit_bindgen::generate!({world:"plugin",path:"wit"});
use serde_json::{Value, json};
struct Colors;
impl Guest for Colors {
    fn run(action: String, context: String) -> Result<String, String> {
        let ctx: Value = serde_json::from_str(&context).map_err(|e| e.to_string())?;
        let ids = ctx["view"]["selection"]
            .as_array()
            .ok_or("Missing selection")?;
        let color = match action.as_str() {
            "colors.set" => ctx["arguments"]["color"]
                .as_str()
                .ok_or("color required")?
                .to_string(),
            "colors.clear" => String::new(),
            _ => return Err("Unknown colors action".into()),
        };
        Ok(json!({"commands":[{"action":"colors_set","args":{"ids":ids,"color":if color.is_empty(){Value::Null}else{json!(color)}}}],"result":{}}).to_string())
    }
}
export!(Colors);
