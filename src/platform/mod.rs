#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Platform {
    #[serde(skip)]
    pub name: String,
    #[serde(skip_serializing_if = "is_empty_table")]
    pub variables: Table,
}

impl Platform {
    pub fn new(name: &str) -> Self {
        Platform {
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub fn from_table(name: &str, table: &Table) -> anyhow::Result<Self> {
        let variables = match table.get("variables") {
            Some(var_block) => var_block
                .as_table()
                .ok_or_else(|| anyhow::anyhow!("The 'variables' field must be a table"))?
                .clone(),
            None => Table::new(),
        };
        Ok(Self {
            name: name.to_string(),
            variables,
        })
    }
}
