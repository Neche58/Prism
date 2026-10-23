use clap::Parser;
use grat_core::taxonomy::loader::TaxonomyDatabase;
use serde::Serialize;
use tabled::Table;
use tabled::Tabled;

#[derive(Parser, Debug)]
#[command(
    about = "Search the taxonomy database for specific error names, categories, or descriptions"
)]
pub struct SearchErrorArgs {
    #[arg(help = "The search query (case-insensitive substring match)")]
    pub query: String,
}

#[derive(Tabled, Serialize)]
struct SearchResultRow {
    #[tabled(rename = "Category")]
    category: String,
    #[tabled(rename = "Code")]
    code: u32,
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Summary")]
    summary: String,
    #[tabled(skip)]
    severity: String,
}

pub async fn run(args: SearchErrorArgs, output_format: &str) -> anyhow::Result<()> {
    let db = TaxonomyDatabase::load_latest()?;

    let results = db.search(&args.query);

    let rows: Vec<SearchResultRow> = results
        .into_iter()
        .map(|entry| SearchResultRow {
            category: entry.category.to_string(),
            code: entry.code,
            name: entry.name.clone(),
            summary: entry.summary.clone(),
            severity: entry.severity.clone(),
        })
        .collect();

    if matches!(
        crate::output::OutputFormat::parse(output_format),
        crate::output::OutputFormat::Json
    ) {
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    if rows.is_empty() {
        println!("No matching entries found for '{}'", args.query);
        return Ok(());
    }

    let table = Table::new(rows);
    println!("{table}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_row() -> SearchResultRow {
        SearchResultRow {
            category: "Budget".to_string(),
            code: 1,
            name: "CPUExceeded".to_string(),
            summary: "CPU limit exceeded".to_string(),
            severity: "Error".to_string(),
        }
    }

    #[test]
    fn json_payload_exposes_expected_fields() {
        let value = serde_json::to_value(sample_row()).expect("row should serialize to JSON");
        let object = value
            .as_object()
            .expect("row should serialize as a JSON object");

        for field in ["category", "code", "name", "summary", "severity"] {
            assert!(
                object.contains_key(field),
                "JSON payload is missing '{field}' field"
            );
        }

        assert_eq!(
            object.len(),
            5,
            "JSON payload should only contain the documented fields"
        );
        assert_eq!(object["category"], serde_json::json!("Budget"));
        assert_eq!(object["code"], serde_json::json!(1));
        assert_eq!(object["name"], serde_json::json!("CPUExceeded"));
        assert_eq!(object["summary"], serde_json::json!("CPU limit exceeded"));
        assert_eq!(object["severity"], serde_json::json!("Error"));
    }

    #[test]
    fn json_payload_is_an_array_of_entries() {
        let payload = vec![sample_row(), sample_row()];
        let value = serde_json::to_value(&payload).expect("rows should serialize to JSON");

        assert!(value.is_array(), "JSON payload should be an array");
        assert_eq!(value.as_array().map(Vec::len), Some(2));
    }

    #[test]
    fn taxonomy_entries_expose_severity_for_json_output() {
        let db = TaxonomyDatabase::load_embedded().expect("embedded taxonomy should load");
        let results = db.search("budget");

        assert!(
            !results.is_empty(),
            "embedded taxonomy should contain budget entries"
        );

        for entry in results {
            assert!(
                !entry.severity.trim().is_empty(),
                "taxonomy entry {} has no severity",
                entry.name
            );
        }
    }
}
