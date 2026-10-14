//! Selectable-text PDF extraction. No renderer, scripts, attachments, or OCR.
use crate::budget::money;
use crate::imports::{
    reported_money, source_date, DateFormat, Fields, Kind, Raw, MAX_BYTES, MAX_ROWS,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::io::{Read, Write};
use zeroize::Zeroizing;

pub const MAX_TEXT: usize = 1024 * 1024;
pub const MAX_PAGES: usize = 100;
pub const MAX_SELECTED: usize = 500;
pub const WORKER_ARG: &str = "--private-pdf-worker-v1";
pub const PDF_ERROR: &str = "This PDF could not be extracted safely. Use a supported selectable-text PDF (at most 2 MiB / 100 pages), or a CSV export. Password-protected, damaged, scanned, or unsupported PDFs cannot be imported in this stage.";
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Locator {
    pub page: u32,
    pub line: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Page {
    pub number: u32,
    pub text: String,
}

struct BoundedText {
    bytes: Vec<u8>,
    remaining: usize,
}
impl Write for BoundedText {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(std::io::Error::other("PDF text limit"));
        }
        self.bytes.extend_from_slice(bytes);
        self.remaining -= bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub fn validate_pages(pages: &[Page]) -> Result<(), String> {
    if pages.is_empty() || pages.len() > MAX_PAGES {
        return Err(PDF_ERROR.into());
    }
    let mut total = 0;
    let mut lines = 0;
    let mut all_lines = 0;
    for (i, page) in pages.iter().enumerate() {
        if page.number as usize != i + 1 {
            return Err(PDF_ERROR.into());
        }
        total += page.text.len();
        if total > MAX_TEXT
            || page
                .text
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        {
            return Err(PDF_ERROR.into());
        }
        all_lines += page.text.lines().count();
        if all_lines > MAX_ROWS * 2 {
            return Err(PDF_ERROR.into());
        }
        for line in page.text.lines().filter(|v| !v.trim().is_empty()) {
            lines += 1;
            if line.len() > 2048 || lines > MAX_ROWS {
                return Err(PDF_ERROR.into());
            }
        }
    }
    if lines == 0 {
        return Err(PDF_ERROR.into());
    }
    Ok(())
}
// This function is used only inside the private process in production.
pub fn extract_pages(bytes: &[u8]) -> Result<Vec<Page>, String> {
    if bytes.len() > MAX_BYTES || !bytes.starts_with(b"%PDF-") {
        return Err(PDF_ERROR.into());
    }
    let doc = lopdf::Document::load_mem(bytes).map_err(|_| PDF_ERROR)?;
    if doc.was_encrypted() {
        return Err(PDF_ERROR.into());
    }
    let page_ids = doc.get_pages();
    if page_ids.is_empty() || page_ids.len() > MAX_PAGES {
        return Err(PDF_ERROR.into());
    }
    let mut pages = Vec::new();
    let mut remaining = MAX_TEXT;
    for number in page_ids.keys() {
        let mut text = BoundedText {
            bytes: Vec::new(),
            remaining,
        };
        {
            let writer: &mut dyn Write = &mut text;
            let mut output = pdf_extract::PlainTextOutput::new(writer);
            pdf_extract::output_doc_page(&doc, &mut output, *number).map_err(|_| PDF_ERROR)?;
        }
        remaining = text.remaining;
        pages.push(Page {
            number: *number,
            text: String::from_utf8(text.bytes).map_err(|_| PDF_ERROR)?,
        });
    }
    validate_pages(&pages)?;
    Ok(pages)
}
pub fn worker() {
    // The parser never initializes Tauri or opens a workspace. No parser panic
    // detail, PDF text, workspace credentials, or inherited secrets are logged.
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(|| {
        let mut bytes = Zeroizing::new(Vec::new());
        std::io::stdin()
            .take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| PDF_ERROR)?;
        extract_pages(&bytes)
    });
    if let Ok(Ok(pages)) = result {
        if serde_json::to_writer(std::io::stdout().lock(), &pages).is_ok() {
            return;
        }
    }
    std::process::exit(2);
}
pub fn records(
    pages: &[Page],
    selected: &[Locator],
    format: &DateFormat,
    inverted: bool,
) -> Result<Vec<Raw>, String> {
    validate_pages(pages)?;
    if selected.is_empty()
        || selected.len() > MAX_SELECTED
        || selected.iter().collect::<HashSet<_>>().len() != selected.len()
    {
        return Err(
            "Select 1–500 distinct extracted transaction lines. Nothing is selected automatically."
                .into(),
        );
    }
    let mut selected = selected.to_vec();
    selected.sort_by_key(|v| (v.page, v.line));
    let lines: BTreeMap<_, _> = pages
        .iter()
        .flat_map(|p| {
            p.text
                .lines()
                .enumerate()
                .map(move |(i, line)| ((p.number, i as u32 + 1), line))
        })
        .collect();
    selected.iter().enumerate().map(|(i, locator)| {
        let text = lines.get(&(locator.page, locator.line)).copied()
            .filter(|s| !s.trim().is_empty()).ok_or("A selected PDF line is unavailable. Inspect the PDF again.")?;
        let words = text.split_whitespace().collect::<Vec<_>>();
        let reported_date = words[0].to_owned();
        let date = source_date(&reported_date, format);
        let tail = words.last().copied().filter(|_| words.len() > 1);
        let amount = tail.and_then(|v| reported_money(v).ok());
        let ambiguous = words.iter().skip(1).take(words.len().saturating_sub(2)).any(|v| reported_money(v).is_ok());
        let amount = if ambiguous { None } else { amount };
        let end = if tail.is_some_and(|v| reported_money(v).is_ok()) { words.len()-1 } else { words.len() };
        let reported_description = words[1..end].join(" ");
        let description = if reported_description.chars().count() <= 500 { reported_description.clone() } else { String::new() };
        let mut issues = vec!["PDF text and line ordering may differ from the visible statement. Verify this page/line and USD currency individually; classify the transaction and assign a category.".into()];
        if date.is_none() { issues.push("The first token is not a valid date in the selected format; correct it from the statement.".into()); }
        if amount.is_none() { issues.push("The final signed amount is missing/invalid, or more than one numeric value is present. Verify the amount individually; debit/credit columns and running-balance tables need later adapters.".into()); }
        if description.is_empty() { issues.push("A verified transaction description is required.".into()); }
        Ok(Raw { page: Some(locator.page), reported_line: Some(text.to_owned()), extraction_method: "pdf-text-lines-v1".into(),
            record_number: i as u32+1, line: u64::from(locator.line), reported_date,
            reported_amount: tail.map(str::to_owned), reported_debit: None, reported_credit: None,
            reported_description, reported_merchant: None, reported_category: None, reported_kind: None,
            reported_reference: None, reported_balance: None,
            extracted: Fields { date, amount: amount.map(|v| money(if inverted {-v} else {v})), description,
                merchant: String::new(), category: String::new(), kind: Kind::Unclassified },
            reference: None, balance: None, issues })
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf_fixture as fixture;
    fn locators(pages: &[Page]) -> Vec<Locator> {
        pages
            .iter()
            .flat_map(|p| {
                p.text
                    .lines()
                    .enumerate()
                    .filter(|(_, s)| s.starts_with("2026-"))
                    .map(|(i, _)| Locator {
                        page: p.number,
                        line: i as u32 + 1,
                    })
            })
            .collect()
    }
    #[test]
    fn actual_pdf_multiple_pages_dates_exact_amounts_missing_and_ambiguous_values() {
        let pages = extract_pages(&fixture::statement()).unwrap();
        assert_eq!(pages.len(), 2);
        let selected = locators(&pages);
        let rows = records(&pages, &selected, &DateFormat::Iso, false).unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].page, Some(1));
        assert_eq!(rows[2].page, Some(2));
        assert_eq!(rows[0].extracted.amount.as_deref(), Some("1000.01"));
        assert_eq!(rows[1].extracted.amount.as_deref(), Some("-25.11"));
        assert!(rows[2].extracted.amount.is_none());
        assert!(rows[3].extracted.amount.is_none());
        assert_eq!(rows[3].reported_amount.as_deref(), Some("900.00"));
        assert_eq!(
            rows[0].reported_line.as_deref(),
            Some("2026-10-01 Fictional payroll 1000.01")
        );
        for r in rows {
            assert_eq!(r.extracted.kind, Kind::Unclassified);
            assert!(r.extracted.category.is_empty());
            assert!(!r.issues.is_empty());
        }
        assert!(records(&pages, &[], &DateFormat::Iso, false).is_err());
        assert!(records(
            &pages,
            &[selected[0].clone(), selected[0].clone()],
            &DateFormat::Iso,
            false
        )
        .is_err());
        assert!(records(
            &pages,
            &[Locator { page: 3, line: 1 }],
            &DateFormat::Iso,
            false
        )
        .is_err());
    }
    #[test]
    fn explicit_day_month_polarity_glyphs_and_inert_text_are_preserved() {
        let pages = vec![Page {
            number: 1,
            text: "31/10/2026 <img onerror=alert(1)> -$5.01\n10/31/2026 Merchant 1.001".into(),
        }];
        let rows = records(
            &pages,
            &[Locator { page: 1, line: 1 }, Locator { page: 1, line: 2 }],
            &DateFormat::Dmy,
            true,
        )
        .unwrap();
        assert_eq!(rows[0].extracted.date.as_deref(), Some("2026-10-31"));
        assert_eq!(rows[0].extracted.amount.as_deref(), Some("5.01"));
        assert!(rows[0].reported_line.as_ref().unwrap().contains("<img"));
        assert!(rows[1].extracted.date.is_none());
        assert!(rows[1].extracted.amount.is_none());
    }
    #[test]
    fn encrypted_empty_scanned_damaged_input_and_output_bounds_fail() {
        assert!(extract_pages(&fixture::protected()).is_err());
        assert!(extract_pages(&fixture::protected_with_password("")).is_err());
        assert!(extract_pages(&fixture::bytes(fixture::document(&[&[]]))).is_err());
        assert!(extract_pages(b"%PDF-1.4\ndamaged").is_err());
        assert!(validate_pages(&[Page {
            number: 1,
            text: "a\n".repeat(MAX_ROWS * 2 + 1)
        }])
        .is_err());
        assert!(validate_pages(&[Page {
            number: 1,
            text: "a".repeat(2049)
        }])
        .is_err());
        assert!(validate_pages(&[Page {
            number: 1,
            text: "a\0b".into()
        }])
        .is_err());
        assert!(validate_pages(&vec![
            Page {
                number: 1,
                text: "A".into()
            };
            101
        ])
        .is_err());
        let mut writer = BoundedText {
            bytes: vec![],
            remaining: 2,
        };
        assert!(writer.write_all(b"abc").is_err());
        assert!(writer.bytes.is_empty());
    }
    #[test]
    fn legacy_csv_provenance_without_pdf_fields_remains_readable() {
        let csv =
            crate::imports::read_csv(b"D,T,A,C,K\n2026-01-01,Synthetic,1,Wages,income\n").unwrap();
        let raw = crate::imports::extract(&csv, &crate::imports::tests::mapping())
            .unwrap()
            .remove(0);
        let mut value = serde_json::to_value(&raw).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("page");
        object.remove("reportedLine");
        assert_eq!(serde_json::from_value::<Raw>(value).unwrap(), raw);
    }
    #[test]
    fn embedded_javascript_is_never_evaluated_or_included_in_text() {
        let mut doc = fixture::document(&[&["2026-10-01 Synthetic safe text 100"]]);
        let root = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        doc.objects.get_mut(&root).unwrap().as_dict_mut().unwrap().set("OpenAction",lopdf::dictionary!{"S"=>"JavaScript","JS"=>lopdf::Object::string_literal("throw new Error('Fictional inert active action')")});
        let pages = extract_pages(&fixture::bytes(doc)).unwrap();
        assert!(pages[0].text.contains("Synthetic safe text"));
        assert!(!pages[0].text.contains("inert active action"));
    }
}
