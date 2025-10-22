//! Database diff tooling for comparing Rust vs Python WeeWX output

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::Command;

/// Database dump for comparison
#[derive(Debug, Clone)]
pub struct DbDump {
    pub tables: HashMap<String, TableDump>,
}

/// Single table dump
#[derive(Debug, Clone)]
pub struct TableDump {
    pub name: String,
    pub rows: Vec<HashMap<String, String>>,
}

impl DbDump {
    /// Create a dump from a MySQL database
    pub async fn from_database(database_url: &str) -> Result<Self> {
        let dump_sql = dump_database(database_url)
            .await
            .context("Failed to dump database")?;

        Self::from_sql(&dump_sql)
    }

    /// Parse a SQL dump into structured format
    pub fn from_sql(sql: &str) -> Result<Self> {
        // Simplified parser - production version would use proper SQL parser
        let mut tables = HashMap::new();

        // Extract table data from INSERT statements
        // This is a simplified version - full implementation would parse CREATE and INSERT

        Ok(Self { tables })
    }

    /// Load a dump from a file
    pub fn from_file(path: &std::path::Path) -> Result<Self> {
        let sql = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read dump file: {:?}", path))?;
        Self::from_sql(&sql)
    }

    /// Save dump to a file
    pub fn to_file(&self, path: &std::path::Path) -> Result<()> {
        let sql = self.to_sql();
        std::fs::write(path, sql)
            .with_context(|| format!("Failed to write dump file: {:?}", path))?;
        Ok(())
    }

    /// Convert dump to SQL
    pub fn to_sql(&self) -> String {
        let mut sql = String::new();

        for (table_name, table) in &self.tables {
            sql.push_str(&format!("-- Table: {}\n", table_name));
            for row in &table.rows {
                sql.push_str(&format!("{:?}\n", row));
            }
            sql.push('\n');
        }

        sql
    }

    /// Compare two dumps and return differences
    pub fn diff(&self, other: &DbDump) -> Vec<String> {
        let mut differences = Vec::new();

        // Check for missing/extra tables
        for table_name in self.tables.keys() {
            if !other.tables.contains_key(table_name) {
                differences.push(format!(
                    "Table '{}' exists in actual but not in expected",
                    table_name
                ));
            }
        }

        for table_name in other.tables.keys() {
            if !self.tables.contains_key(table_name) {
                differences.push(format!(
                    "Table '{}' exists in expected but not in actual",
                    table_name
                ));
            }
        }

        // Compare table contents
        for (table_name, actual_table) in &self.tables {
            if let Some(expected_table) = other.tables.get(table_name) {
                let table_diffs = compare_tables(actual_table, expected_table);
                differences.extend(table_diffs);
            }
        }

        differences
    }
}

/// Compare two table dumps
fn compare_tables(actual: &TableDump, expected: &TableDump) -> Vec<String> {
    let mut differences = Vec::new();

    if actual.rows.len() != expected.rows.len() {
        differences.push(format!(
            "Table '{}': row count mismatch (actual: {}, expected: {})",
            actual.name,
            actual.rows.len(),
            expected.rows.len()
        ));
    }

    // Compare row by row (simplified - production would do smarter matching)
    let min_rows = actual.rows.len().min(expected.rows.len());
    for i in 0..min_rows {
        let actual_row = &actual.rows[i];
        let expected_row = &expected.rows[i];

        for (key, actual_val) in actual_row {
            if let Some(expected_val) = expected_row.get(key) {
                if actual_val != expected_val {
                    // Special handling for floating point comparison
                    if let (Ok(a), Ok(e)) = (actual_val.parse::<f64>(), expected_val.parse::<f64>())
                    {
                        if (a - e).abs() > 0.0001 {
                            differences.push(format!(
                                "Table '{}', row {}, column '{}': value mismatch (actual: {}, expected: {})",
                                actual.name, i, key, actual_val, expected_val
                            ));
                        }
                    } else if actual_val != expected_val {
                        differences.push(format!(
                            "Table '{}', row {}, column '{}': value mismatch (actual: {}, expected: {})",
                            actual.name, i, key, actual_val, expected_val
                        ));
                    }
                }
            } else {
                differences.push(format!(
                    "Table '{}', row {}: column '{}' exists in actual but not in expected",
                    actual.name, i, key
                ));
            }
        }

        for key in expected_row.keys() {
            if !actual_row.contains_key(key) {
                differences.push(format!(
                    "Table '{}', row {}: column '{}' exists in expected but not in actual",
                    actual.name, i, key
                ));
            }
        }
    }

    differences
}

/// Dump a MySQL database using mysqldump
async fn dump_database(database_url: &str) -> Result<String> {
    // Parse database URL
    let url = url::Url::parse(database_url).context("Invalid database URL")?;

    let host = url.host_str().unwrap_or("localhost");
    let database = url.path().trim_start_matches('/');
    let username = if url.username().is_empty() {
        "root"
    } else {
        url.username()
    };

    // Run mysqldump
    let output = Command::new("mysqldump")
        .args(&[
            "-h",
            host,
            "-u",
            username,
            "--skip-comments",
            "--skip-extended-insert",
            "--compact",
            database,
        ])
        .output()
        .context("Failed to run mysqldump")?;

    if !output.status.success() {
        anyhow::bail!(
            "mysqldump failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8(output.stdout)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_dump_creation() {
        let dump = DbDump {
            tables: HashMap::new(),
        };

        assert_eq!(dump.tables.len(), 0);
    }

    #[test]
    fn test_table_comparison() {
        let table1 = TableDump {
            name: "test".to_string(),
            rows: vec![],
        };

        let table2 = TableDump {
            name: "test".to_string(),
            rows: vec![],
        };

        let diffs = compare_tables(&table1, &table2);
        assert_eq!(diffs.len(), 0);
    }
}
