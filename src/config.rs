use anyhow::Result;
use toml::Table;
use std::fs;

use crate::error::AcceleratorError;

pub const CONFIG_LOCATION: &str = "garrysmod/lua/bin/accelerator.toml";

pub struct Config(toml::Table);

impl Config {
    pub fn from_file(location: &str, target: &str) -> Result<Self> {
        let content = fs::read_to_string(location)?;
        let table = content.parse::<Table>()?;
    
        let sigs = table.get(target).ok_or(AcceleratorError::EntryMissing(target.to_string()))?
                .as_table().ok_or(AcceleratorError::EntryInvalid(target.to_string()))?;
        Ok(Self { 0: sigs.clone() })
    }

    pub fn get_value(&self, key: &str) -> Result<String> {
        let entry = self.0.get(key).ok_or(AcceleratorError::EntryMissing(key.to_string()))?;
        let value = entry.as_str().ok_or(AcceleratorError::EntryInvalid(key.to_string()))?.to_string();
        
        Ok(value)
    }
}