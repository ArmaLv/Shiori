use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartRule {
    pub field: String,
    pub operator: String,
    pub value: String,
    pub match_type: String,
}

fn main() {
    let json = r#"[{"field":"author","operator":"contains","value":"foo","matchType":"all"}]"#;
    let res: Result<Vec<SmartRule>, _> = serde_json::from_str(json);
    println!("{:?}", res);
}
