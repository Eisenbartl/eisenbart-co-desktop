//! Offline printable internal records. Money strings and integer chart scales come
//! only from a validated immutable snapshot; floating point is typography only.
use crate::assessments::Record;
use lopdf::{
    content::{Content, Operation},
    dictionary, Document, Object, Stream,
};
use std::{collections::BTreeMap, io::Write, path::Path};
const FONT: &[u8] = include_bytes!("../assets/fonts/OpenSans-Regular.ttf");
const WIDTH: f32 = 516.0;
const MAX_PAGES: usize = 1000;
const MAX_BYTES: usize = 32 * 1024 * 1024;
const ERROR: &str = "PDF export could not finish. Your saved assessment is unchanged.";
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Export {
    pub path: String,
    pub pages: usize,
    pub record_id: String,
}
struct Layout {
    pages: Vec<Vec<Operation>>,
    y: f32,
    face: ttf_parser::Face<'static>,
    chars: BTreeMap<char, u16>,
}
impl Layout {
    fn new() -> Result<Self, String> {
        Ok(Self {
            pages: vec![vec![]],
            y: 730.0,
            face: ttf_parser::Face::parse(FONT, 0).map_err(|_| ERROR)?,
            chars: BTreeMap::new(),
        })
    }
    fn room(&mut self, height: f32) -> Result<(), String> {
        if self.y - height < 62.0 {
            if self.pages.len() >= MAX_PAGES {
                return Err("This report exceeds the 1,000-page export limit. Use a smaller assessment record.".into());
            }
            self.pages.push(vec![]);
            self.y = 730.0;
        }
        Ok(())
    }
    fn advance(&self, c: char, size: f32) -> Result<f32, String> {
        // This renderer has no shaping engine. Never silently lose unsupported text.
        if matches!(c,'\u{0300}'..='\u{036f}'|'\u{200b}'..='\u{200f}'|'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}')
            || c.is_control()
        {
            return Err(format!("PDF typography cannot represent U+{:04X}. Use supported plain text or precomposed accented letters in a new assessment.",c as u32));
        }
        let g=self.face.glyph_index(c).ok_or_else(||format!("The bundled PDF font does not support U+{:04X}. Use supported text in a new assessment; the saved record is unchanged.",c as u32))?;
        Ok(
            f32::from(self.face.glyph_hor_advance(g).ok_or(ERROR)?) * size
                / f32::from(self.face.units_per_em()),
        )
    }
    fn encode(&mut self, text: &str) -> Result<Vec<u8>, String> {
        let mut b = vec![];
        for c in text.chars() {
            self.advance(c, 10.0)?;
            let next = u16::try_from(self.chars.len() + 1).map_err(|_| ERROR)?;
            let id = *self.chars.entry(c).or_insert(next);
            b.extend(id.to_be_bytes());
        }
        Ok(b)
    }
    fn line(&mut self, text: &str, size: f32) -> Result<(), String> {
        self.room(size + 7.0)?;
        let encoded = self.encode(text)?;
        self.pages.last_mut().unwrap().extend([
            Operation::new("BT", vec![]),
            Operation::new("rg", vec![0.10.into(), 0.18.into(), 0.24.into()]),
            Operation::new("Tf", vec!["F1".into(), size.into()]),
            Operation::new(
                "Tm",
                vec![
                    1.into(),
                    0.into(),
                    0.into(),
                    1.into(),
                    48.into(),
                    self.y.into(),
                ],
            ),
            Operation::new(
                "Tj",
                vec![Object::String(encoded, lopdf::StringFormat::Hexadecimal)],
            ),
            Operation::new("ET", vec![]),
        ]);
        self.y -= size + 7.0;
        Ok(())
    }
    fn text(&mut self, text: &str, size: f32) -> Result<(), String> {
        // Preserve newlines, normalize only CRLF and tabs for printable layout. Split
        // long tokens by actual glyph width so hashes and long source lines never clip.
        for paragraph in text.replace("\r\n", "\n").replace('\t', "    ").split('\n') {
            let mut line = String::new();
            let mut width = 0.0;
            for c in paragraph.chars() {
                let w = self.advance(c, size)?;
                if width + w > WIDTH && !line.is_empty() {
                    if let Some(i) = line.rfind(' ') {
                        let tail = line[i + 1..].to_owned();
                        self.line(line[..i].trim_end(), size)?;
                        line = tail;
                        width = line
                            .chars()
                            .map(|c| self.advance(c, size))
                            .collect::<Result<Vec<_>, _>>()?
                            .iter()
                            .sum();
                    } else {
                        self.line(&line, size)?;
                        line.clear();
                        width = 0.0;
                    }
                }
                if width + w > WIDTH && !line.is_empty() {
                    self.line(&line, size)?;
                    line.clear();
                    width = 0.0;
                }
                line.push(c);
                width += w;
            }
            self.line(&line, size)?;
        }
        Ok(())
    }
    fn section(&mut self, title: &str) -> Result<(), String> {
        self.room(70.0)?;
        self.y -= 12.0;
        self.text(title, 14.0)
    }
    fn value(&mut self, label: &str, value: &str) -> Result<(), String> {
        self.text(&format!("{label}: {value}"), 10.0)
    }
    fn bar(&mut self, label: &str, value: &str, scale: u16) -> Result<(), String> {
        self.room(46.0)?;
        self.value(label, value)?;
        self.pages.last_mut().unwrap().extend([
            Operation::new("rg", vec![0.15.into(), 0.42.into(), 0.52.into()]),
            Operation::new(
                "re",
                vec![
                    48.into(),
                    (self.y - 1.0).into(),
                    (f32::from(scale) * WIDTH / 1000.0).into(),
                    6.into(),
                ],
            ),
            Operation::new("f", vec![]),
        ]);
        self.y -= 13.0;
        Ok(())
    }
    fn finish(mut self) -> Result<(Vec<u8>, usize), String> {
        let count = self.pages.len();
        for i in 0..count {
            let text = format!("INTERNAL / INCOMPLETE  |  Page {} of {count}", i + 1);
            let encoded = self.encode(&text)?;
            self.pages[i].extend([
                Operation::new("BT", vec![]),
                Operation::new("rg", vec![0.25.into(), 0.25.into(), 0.25.into()]),
                Operation::new("Tf", vec!["F1".into(), 8.into()]),
                Operation::new(
                    "Tm",
                    vec![1.into(), 0.into(), 0.into(), 1.into(), 48.into(), 36.into()],
                ),
                Operation::new(
                    "Tj",
                    vec![Object::String(encoded, lopdf::StringFormat::Hexadecimal)],
                ),
                Operation::new("ET", vec![]),
            ]);
        }
        let mut d = Document::with_version("1.7");
        let pages = d.new_object_id();
        let fontfile = d.add_object(Stream::new(
            dictionary! {"Length1"=>FONT.len() as i64},
            FONT.to_vec(),
        ));
        let box_ = self.face.global_bounding_box();
        let units = f32::from(self.face.units_per_em());
        let unit = |n: i16| f32::from(n) * 1000.0 / units;
        let descriptor=d.add_object(dictionary!{"Type"=>"FontDescriptor","FontName"=>"OpenSans","Flags"=>32,"FontBBox"=>vec![unit(box_.x_min).into(),unit(box_.y_min).into(),unit(box_.x_max).into(),unit(box_.y_max).into()],"ItalicAngle"=>0,"Ascent"=>unit(self.face.ascender()),"Descent"=>unit(self.face.descender()),"CapHeight"=>unit(self.face.capital_height().unwrap_or(self.face.ascender())),"StemV"=>80,"FontFile2"=>fontfile});
        let mut mapping = vec![0u8; (self.chars.len() + 1) * 2];
        let mut widths = vec![];
        let mut pairs = vec![];
        for (c, id) in &self.chars {
            let glyph = self.face.glyph_index(*c).ok_or(ERROR)?;
            mapping[usize::from(*id) * 2..usize::from(*id) * 2 + 2]
                .copy_from_slice(&glyph.0.to_be_bytes());
            widths.push((id, self.advance(*c, 1000.0)?));
            let mut buf = [0; 2];
            let utf = c
                .encode_utf16(&mut buf)
                .iter()
                .map(|v| format!("{v:04X}"))
                .collect::<String>();
            pairs.push(format!("<{id:04X}> <{utf}>\n"));
        }
        widths.sort_by_key(|(id, _)| **id);
        let widths = widths
            .into_iter()
            .map(|(_, w)| w.into())
            .collect::<Vec<Object>>();
        let map = d.add_object(Stream::new(dictionary! {}, mapping));
        let cid=d.add_object(dictionary!{"Type"=>"Font","Subtype"=>"CIDFontType2","BaseFont"=>"OpenSans","CIDSystemInfo"=>dictionary!{"Registry"=>Object::string_literal("Adobe"),"Ordering"=>Object::string_literal("Identity"),"Supplement"=>0},"FontDescriptor"=>descriptor,"CIDToGIDMap"=>map,"W"=>vec![1.into(),Object::Array(widths)]});
        let mut cmap=String::from("/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def /CMapName /CounselorUnicode def /CMapType 2 def 1 begincodespacerange <0000> <FFFF> endcodespacerange\n");
        for chunk in pairs.chunks(100) {
            cmap.push_str(&format!(
                "{} beginbfchar\n{}endbfchar\n",
                chunk.len(),
                chunk.concat()
            ));
        }
        cmap.push_str("endcmap CMapName currentdict /CMap defineresource pop end end");
        let unicode = d.add_object(Stream::new(dictionary! {}, cmap.into_bytes()));
        let font=d.add_object(dictionary!{"Type"=>"Font","Subtype"=>"Type0","BaseFont"=>"OpenSans","Encoding"=>"Identity-H","DescendantFonts"=>vec![Object::Reference(cid)],"ToUnicode"=>unicode});
        let mut kids = vec![];
        for operations in self.pages {
            let content = Content { operations }.encode().map_err(|_| ERROR)?;
            let stream = d.add_object(Stream::new(dictionary! {}, content));
            let page=d.add_object(dictionary!{"Type"=>"Page","Parent"=>pages,"Contents"=>stream,"MediaBox"=>vec![0.into(),0.into(),612.into(),792.into()],"Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>font}}});
            kids.push(Object::Reference(page));
        }
        d.objects.insert(
            pages,
            dictionary! {"Type"=>"Pages","Kids"=>kids,"Count"=>count as i64}.into(),
        );
        let catalog = d.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
        d.trailer.set("Root", catalog);
        let info=d.add_object(dictionary!{"Producer"=>Object::string_literal(concat!("Eisenbart Counseling Suite ",env!("CARGO_PKG_VERSION"))),"Title"=>Object::string_literal("Internal counselor assessment - incomplete information")});
        d.trailer.set("Info", info);
        d.compress();
        let mut bytes = vec![];
        d.save_to(&mut bytes).map_err(|_| ERROR)?;
        if bytes.len() > MAX_BYTES {
            return Err("PDF exceeds the 32 MiB export limit.".into());
        }
        Ok((bytes, count))
    }
}
fn usd(v: &str) -> String {
    format!("USD {v}")
}
fn optional(v: &Option<String>) -> String {
    v.as_deref()
        .map(usd)
        .unwrap_or_else(|| "Unknown / unavailable".into())
}
fn tag<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| "Unknown".into())
}
pub fn render(r: &Record, exported_at: &str) -> Result<(Vec<u8>, usize), String> {
    let p = &r.payload;
    crate::assessments::validate(p)?;
    let a = &p.assessment;
    let mut l = Layout::new()?;
    l.text(&p.workspace.organization_name, 18.0)?;
    l.text(&p.title, 16.0)?;
    l.text(
        "IMMUTABLE INTERNAL ASSESSMENT / INCOMPLETE INFORMATION",
        10.0,
    )?;
    l.value(
        "Client",
        &format!("{} {}", p.client.first_name, p.client.last_name),
    )?;
    l.value("Counselor", &p.workspace.counselor_name)?;
    l.value(
        "Assessment period",
        &format!("{} to {}", a.period_start, a.period_end),
    )?;
    l.value("Recorded at (UTC)", &p.created_at)?;
    l.value("Exported at (UTC)", exported_at)?;
    l.value(
        "Calculation",
        &format!("{} / recorded app {}", p.calculator, p.app_version),
    )?;
    l.text(if r.needs_review{"NEEDS REVIEW: source, client or firm context changed. Historical figures are preserved."}else{"Recorded context matches the current source and client profile at export."},10.0)?;
    for reason in &r.review_reasons {
        l.text(reason, 10.0)?;
    }
    l.value("Household size", &p.client.household_size.to_string())?;
    l.value(
        "Recorded goals",
        if p.client.goals.is_empty() {
            "None recorded"
        } else {
            &p.client.goals
        },
    )?;
    l.section("Basis, exclusions and missing information")?;
    l.text("Approved imported activity only. Statement coverage remains unverified. Pending, NeedsClarification, Rejected, undated and out-of-period records do not supply financial amounts. Internal assessment only; no finalized client action plan, selected planning scenarios, debt APR/repayment terms, budget agreement or saved session notes are included. Counselor text is advice, not approved transaction data. Balance facts are separately dated evidence; transaction running balances do not verify assets.",10.0)?;
    l.value(
        "Record counts within period",
        &format!(
            "Approved {} / Rejected {} / Pending {} / NeedsClarification {}",
            a.totals.approved, a.totals.rejected, a.totals.pending, a.totals.needs_clarification
        ),
    )?;
    l.value("Undated records excluded", &a.undated_records.to_string())?;
    l.value(
        "Latest approved date",
        a.latest_approved_date.as_deref().unwrap_or("Unknown"),
    )?;
    l.value("Outdated period", if a.outdated { "Yes" } else { "No" })?;
    for warning in a.warnings.iter().chain(a.totals.warnings.iter()) {
        l.text(warning, 10.0)?;
    }
    l.section("Approved income, spending and cash flow")?;
    for (label, v) in [
        ("Income receipts", &a.totals.income),
        ("Gross spending", &a.totals.spending),
        ("Expense refunds", &a.totals.refunds),
        ("Net spending", &a.totals.net_spending),
        ("Operating cash flow", &a.operating_cashflow),
        ("Combined account movement", &a.totals.net_cash),
    ] {
        l.value(label, &usd(v))?;
    }
    for (id, v) in &a.totals.account_net {
        l.value(
            &format!(
                "Account movement: {}",
                p.basis
                    .accounts
                    .iter()
                    .find(|x| &x.id == id)
                    .map(|x| x.label.as_str())
                    .unwrap_or("Unknown")
            ),
            &usd(v),
        )?;
    }
    l.section("Monthly activity (income / gross spending)")?;
    l.text("Bars use a common captured scale across months. Cash flow excludes transfer movements; account movement includes them. Negative cash flow remains signed in the figures.",10.0)?;
    for m in &a.months {
        l.bar(
            &format!("{} income", m.month),
            &usd(&m.totals.income),
            m.income_scale,
        )?;
        l.bar(
            &format!("{} spending", m.month),
            &usd(&m.totals.spending),
            m.spending_scale,
        )?;
        l.value("Monthly operating cash flow", &usd(&m.operating_cashflow))?;
    }
    l.section("Gross spending by category")?;
    for c in &a.categories {
        l.bar(&c.label, &usd(&c.spending), c.scale)?;
    }
    if a.categories.is_empty() {
        l.text("No approved spending categories in this period.", 10.0)?;
    }
    let f = &a.facts_metrics;
    l.section("Verified classifications and dated balance facts")?;
    for (label, v) in [
        ("Necessary spending", &f.necessary_spending),
        ("Discretionary spending", &f.discretionary_spending),
        ("Unclassified spending", &f.unclassified_spending),
        ("Classified take-home pay", &f.take_home_pay),
        ("Unclassified income", &f.unclassified_income),
    ] {
        l.value(label, &usd(v))?;
    }
    l.value(
        "Inactive prior classifications",
        &f.stale_classifications.to_string(),
    )?;
    l.value("Balance date", f.as_of.as_deref().unwrap_or("Unknown"))?;
    l.value(
        "Inventory complete / date aligned / outdated",
        &format!(
            "{} / {} / {}",
            f.balance_complete, f.balance_aligned, f.balance_outdated
        ),
    )?;
    for (label, v) in [
        ("Cash", &f.cash),
        ("Accessible savings", &f.accessible_savings),
        ("Restricted savings", &f.restricted_savings),
        ("Debt", &f.debt),
        ("Estimated net worth at balance date", &f.net_worth),
        (
            "Monthly necessary spending estimate",
            &f.monthly_necessary_estimate,
        ),
    ] {
        l.value(label, &optional(v))?;
    }
    l.value(
        "Accessible savings coverage (months; estimate)",
        f.emergency_months_estimate
            .as_deref()
            .unwrap_or("Unavailable"),
    )?;
    l.value(
        "Debt payments / recorded take-home pay (percent; estimate)",
        f.take_home_payment_percent_estimate
            .as_deref()
            .unwrap_or("Unavailable"),
    )?;
    l.text("Coverage uses accessible savings only. Payment ratio uses recorded take-home pay, not gross lender income or underwriting DTI. Estimates depend on completeness, dates and active approved classifications; missing values remain unknown.",10.0)?;
    for (title, text) in [
        ("Counselor findings", &p.findings),
        ("Counselor recommendations", &p.recommendations),
        ("Follow-up arrangements", &p.follow_up),
    ] {
        l.section(title)?;
        l.text(
            if text.is_empty() {
                "None recorded."
            } else {
                text
            },
            10.0,
        )?;
    }
    l.section("Captured balance inventory and evidence")?;
    if let Some(data) = &p.basis.financial_facts.data {
        for b in &data.balances {
            l.value(
                &format!("{} / {}", b.label, tag(&b.kind)),
                &optional(&b.amount),
            )?;
            l.value("Monthly payment", &optional(&b.monthly_payment))?;
            l.value("Evidence", &b.evidence)?;
            l.value(
                "Document reference",
                b.document_id
                    .as_deref()
                    .unwrap_or("Other reviewed evidence"),
            )?;
        }
    } else {
        l.text("No verified balance inventory recorded.", 10.0)?;
    }
    l.section("Captured document sources and discrepancies")?;
    for d in &p.basis.documents {
        l.value(
            "Source",
            &format!(
                "{} / {}",
                d.file_name,
                p.basis
                    .accounts
                    .iter()
                    .find(|a| a.id == d.account_id)
                    .map(|a| a.label.as_str())
                    .unwrap_or("Unknown")
            ),
        )?;
        l.value("Document ID", &d.id)?;
        l.value("SHA-256", &d.sha256)?;
        l.value(
            "Entered statement period",
            &format!(
                "{} to {}",
                d.input.period_start.as_deref().unwrap_or("Unknown"),
                d.input.period_end.as_deref().unwrap_or("Unknown")
            ),
        )?;
        l.value(
            "Original retained in encrypted workspace",
            if d.source_retained {
                "Yes (not attached to this PDF)"
            } else {
                "No"
            },
        )?;
        l.value(
            "Whole-source closing difference",
            &optional(&d.source_difference),
        )?;
        l.value(
            "Approved closing difference",
            &optional(&d.approved_difference),
        )?;
        if let Some(b) = &d.balance_reconciliation {
            l.value(
                "Running-balance differences (source / approved)",
                &format!("{} / {}", b.source_differences, b.approved_differences),
            )?;
        }
        for w in &d.warnings {
            l.text(w, 10.0)?;
        }
    }
    l.section("Captured transaction evidence / all dates and statuses")?;
    for t in &p.basis.records {
        l.value(
            "Transaction",
            &format!(
                "{} / {} / {} / {}",
                t.fields.date.as_deref().unwrap_or("Unknown date"),
                t.fields.description,
                tag(&t.status),
                optional(&t.fields.amount)
            ),
        )?;
        l.value(
            "Category / kind / merchant",
            &format!(
                "{} / {} / {}",
                t.fields.category,
                tag(&t.fields.kind),
                t.fields.merchant
            ),
        )?;
        l.value(
            "Record / source / account",
            &format!("{} / {} / {}", t.id, t.document_id, t.account_id),
        )?;
        l.value(
            "Source locator",
            &format!(
                "record {} / line {} / page {}",
                t.raw.record_number,
                t.raw.line,
                t.raw
                    .page
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "CSV".into())
            ),
        )?;
        l.value(
            "Original reported date / description / amount",
            &format!(
                "{} / {} / {}",
                t.raw.reported_date,
                t.raw.reported_description,
                t.raw.reported_amount.as_deref().unwrap_or("Unknown")
            ),
        )?;
        if let Some(h) = p.basis.history.iter().rev().find(|h| h.record_id == t.id) {
            l.value(
                "Latest captured decision",
                &format!(
                    "{} / {} / {} / {}",
                    h.id, h.reviewed_at, h.counselor_name, h.reason
                ),
            )?;
        }
        for w in &t.warnings {
            l.text(w, 10.0)?;
        }
    }
    l.section("Record integrity and privacy")?;
    l.value("Record ID", &p.id)?;
    l.value("Captured source revision", &a.revision.to_string())?;
    l.value("Stored record SHA-256", &r.digest)?;
    l.text("This checksum checks the stored record bytes, not this exported file, and is not a digital signature or certification. This local PDF contains readable client information outside workspace encryption. It contains captured evidence references, not attached original statement files. Keep it in an appropriate private location. Review the original sources and assumptions before acting on incomplete information.",10.0)?;
    l.finish()
}
pub fn save(r: &Record, path: &str, workspace: &Path) -> Result<Export, String> {
    let p = Path::new(path);
    if !p.is_absolute()
        || p.extension()
            .and_then(|x| x.to_str())
            .map(|s| !s.eq_ignore_ascii_case("pdf"))
            .unwrap_or(true)
    {
        return Err("Choose a full local path ending in .pdf.".into());
    }
    let parent = p
        .parent()
        .ok_or("Choose a local PDF folder.")?
        .canonicalize()
        .map_err(|_| "The PDF folder does not exist or is unavailable.")?;
    let root = workspace.canonicalize().map_err(|_| ERROR)?;
    if parent.starts_with(root) {
        return Err("Save exported PDFs outside the encrypted workspace data folder.".into());
    }
    let name = p.file_name().ok_or("Choose a PDF filename.")?;
    let target = parent.join(name);
    if target.try_exists().map_err(|_| ERROR)? {
        return Err("A file already exists at that path. Choose a new PDF filename; exports never overwrite files.".into());
    }
    let (bytes, pages) = render(r, &chrono::Utc::now().to_rfc3339())?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(&parent).map_err(|_| "The PDF folder is not writable.")?;
    temporary
        .write_all(&bytes)
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|_| ERROR)?;
    temporary.persist_noclobber(&target).map_err(|e| {
        if e.error.kind() == std::io::ErrorKind::AlreadyExists {
            "A file already exists at that path. Choose a new PDF filename."
        } else {
            ERROR
        }
    })?;
    Ok(Export {
        path: target.to_string_lossy().into_owned(),
        pages,
        record_id: r.payload.id.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typography_wraps_long_tokens_within_letter_page_margins_and_fails_at_limit() {
        let mut l = Layout::new().unwrap();
        l.text(&"X".repeat(8000), 10.0).unwrap();
        l.text("Last marker", 10.0).unwrap();
        assert!(l.pages.len() > 1);
        for page in &l.pages {
            for op in page {
                if op.operator == "Tj" {
                    let bytes = op.operands[0].as_str().unwrap();
                    let width: f32 = bytes
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|b| {
                            let id = u16::from_be_bytes([b[0], b[1]]);
                            let c = *l.chars.iter().find(|(_, v)| **v == id).unwrap().0;
                            l.advance(c, 10.0).unwrap()
                        })
                        .sum();
                    assert!(width <= WIDTH + 0.01);
                }
                if op.operator == "Tm" {
                    assert!(op.operands[5].as_float().unwrap() >= 62.0);
                }
            }
        }
        l.pages.resize(MAX_PAGES, vec![]);
        l.y = 63.0;
        assert!(l
            .text("Cannot fit", 10.0)
            .unwrap_err()
            .contains("1,000-page"));
        assert!(Layout::new().unwrap().text("Cafe\u{0301}", 10.0).is_err());
    }
}
