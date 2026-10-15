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
    fn finish(self) -> Result<(Vec<u8>, usize), String> {
        self.finish_labeled(
            "INTERNAL / INCOMPLETE",
            "Internal counselor assessment - incomplete information",
        )
    }
    fn finish_labeled(mut self, label: &str, title: &str) -> Result<(Vec<u8>, usize), String> {
        let count = self.pages.len();
        for i in 0..count {
            let text = format!("{label}  |  Page {} of {count}", i + 1);
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
        let info=d.add_object(dictionary!{"Producer"=>Object::string_literal(concat!("Eisenbart Counseling Suite ",env!("CARGO_PKG_VERSION"))),"Title"=>Object::string_literal(title)});
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
fn scenario_income(p: &crate::planning::Plans) -> Option<&str> {
    let income = p
        .budget
        .lines
        .iter()
        .filter(|l| l.kind == crate::budget::Kind::Income)
        .collect::<Vec<_>>();
    if !income.is_empty() && income.iter().all(|l| l.amount.is_some()) {
        Some(&p.budget.summary.monthly_income)
    } else {
        None
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
fn plan_term<T: serde::Serialize>(value: &T) -> String {
    let mut result = String::new();
    for (i, c) in tag(value).chars().enumerate() {
        if i == 0 {
            result.extend(c.to_uppercase());
        } else {
            if c.is_uppercase() {
                result.push(' ');
            }
            result.extend(c.to_lowercase());
        }
    }
    result
}
fn planning_pdf(l: &mut Layout, s: &crate::planning::Snapshot) -> Result<(), String> {
    let p = &s.plans;
    let b = &p.budget;
    let d = &p.debt;
    let v = &p.savings;
    let c = &p.cashflow;
    l.section("Captured planning scenario / hypothetical")?;
    l.value("Scenario", &s.label)?;
    l.value(
        "Assumptions",
        if s.assumptions.is_empty() {
            "None recorded"
        } else {
            &s.assumptions
        },
    )?;
    l.value(
        "Captured plan versions",
        &format!(
            "Budget {} / debt {} / savings {} / cash flow {}",
            s.revisions.budget, s.revisions.debt, s.revisions.savings, s.revisions.cashflow
        ),
    )?;
    l.text("Entered planning assumptions and forecasts are separate from approved historical facts. Later plan edits do not rewrite these captured values. This is not a final agreed client action plan. Cash-flow schedules are independently entered and are not automatically synchronized to budget, debt or savings amounts. This PDF prints assumptions and summary milestones; full model schedules remain captured in the encrypted record and its viewer.",10.0)?;
    for (label, time) in [
        ("Budget saved (UTC)", &b.updated_at),
        ("Debt saved (UTC)", &d.updated_at),
        ("Savings saved (UTC)", &v.updated_at),
        ("Cash flow saved (UTC)", &c.updated_at),
    ] {
        l.value(label, time.as_deref().unwrap_or("Not saved"))?;
    }
    l.section("Entered monthly budget / proposed allocations")?;
    for (label, val) in [
        ("Monthly income", &b.summary.monthly_income),
        ("Necessary spending", &b.summary.necessary_expenses),
        ("Discretionary spending", &b.summary.discretionary_expenses),
        (
            "Surplus before allocations",
            &b.summary.surplus_before_allocations,
        ),
        ("Proposed savings", &b.summary.proposed_savings),
        ("Planned debt payments", &b.summary.planned_debt_payments),
        ("Unallocated", &b.summary.unallocated),
    ] {
        l.value(label, &usd(val))?;
    }
    for (i, line) in b.lines.iter().enumerate() {
        l.value(
            &line.label,
            &format!(
                "{} / {} / entered {} / monthly {} / {}",
                plan_term(&line.kind),
                plan_term(&line.frequency),
                optional(&line.amount),
                optional(&b.summary.monthly_amounts[i]),
                if line.estimated {
                    "Estimated"
                } else if line.amount.is_none() {
                    "Missing"
                } else {
                    "Entered"
                }
            ),
        )?;
    }
    for w in &b.summary.warnings {
        l.text(w, 10.0)?;
    }
    l.section("Captured debt terms and selected repayment strategy")?;
    l.value(
        "Selected strategy",
        match s.debt_strategy {
            crate::planning::Strategy::None => "No repayment projection selected",
            crate::planning::Strategy::Snowball => "Snowball",
            crate::planning::Strategy::Avalanche => "Avalanche",
        },
    )?;
    if let Some(data) = &d.data {
        l.value("Debt start month", &data.start_month)?;
        l.value(
            "Entered monthly debt payment",
            &optional(&data.monthly_payment),
        )?;
        for a in &data.accounts {
            l.value(
                "Debt account",
                &format!("{} / {}", a.label, plan_term(&a.debt_type)),
            )?;
            l.value(
                "Balance / APR",
                &format!(
                    "{} / {}",
                    optional(&a.balance),
                    a.apr
                        .as_deref()
                        .map(|v| format!("{v}%"))
                        .unwrap_or_else(|| "Unknown".into())
                ),
            )?;
            l.value(
                "Minimum payment / monthly fee",
                &format!(
                    "{} / {}",
                    optional(&a.minimum_payment),
                    optional(&a.monthly_fee)
                ),
            )?;
            l.value(
                "Interest model / terms reviewed",
                &format!(
                    "{} / {}",
                    plan_term(&a.interest_model),
                    if a.verified { "Yes" } else { "No" }
                ),
            )?;
            l.value(
                "Terms note",
                if a.terms_note.is_empty() {
                    "None recorded"
                } else {
                    &a.terms_note
                },
            )?;
        }
        for x in &data.extras {
            l.value(
                "Extra payment",
                &format!("{} / {}", x.month, usd(&x.amount)),
            )?;
        }
        for x in &data.payment_changes {
            l.value(
                "Changed monthly payment",
                &format!("{} / {}", x.month, usd(&x.amount)),
            )?;
        }
    } else {
        l.text(
            "No saved debt plan. Repayment terms and outcomes remain missing.",
            10.0,
        )?;
    }
    if let Some(cmp) = &d.comparison {
        for w in &cmp.warnings {
            l.text(w, 10.0)?;
        }
    }
    if let Some(result) = crate::planning::projection(s) {
        l.value("Repayment outcome", &plan_term(&result.status))?;
        l.value(
            "Projected payoff month",
            result.payoff_month.as_deref().unwrap_or("Not projected"),
        )?;
        for (label, val) in [
            ("Projected total interest", &result.total_interest),
            ("Projected total fees", &result.total_fees),
            ("Projected total paid", &result.total_paid),
            ("Projected remaining debt", &result.remaining_balance),
        ] {
            l.value(label, &usd(val))?;
        }
        if let Some(data) = &d.data {
            for (i, a) in data.accounts.iter().enumerate() {
                l.value(
                    &format!("Projected payoff: {}", a.label),
                    result.account_payoff_months[i]
                        .as_deref()
                        .unwrap_or("Not projected"),
                )?;
            }
        }
        for w in &result.warnings {
            l.text(w, 10.0)?;
        }
        l.text("The supported repayment projection uses fixed monthly interest. Missing lender dates and daily, variable or promotional terms are not inferred.",10.0)?;
    }
    l.section("Captured savings targets / primary contribution")?;
    if let (Some(data), Some(result)) = (&v.data, &v.summary) {
        l.value("Savings start month", &data.start_month)?;
        l.value("Target coverage (months)", &data.target_months.to_string())?;
        for (label, val) in [
            ("Entered liquid savings", &data.liquid_savings),
            ("Entered restricted savings", &data.restricted_savings),
            ("Obligations reserved", &data.obligations_reserved),
            ("Accessible emergency pool", &result.accessible_emergency),
            ("Emergency target", &result.emergency_target),
            (
                "Primary emergency contribution",
                &data.emergency_contribution,
            ),
            (
                "Requested monthly savings",
                &result.requested_monthly_savings,
            ),
        ] {
            l.value(label, &optional(val))?;
        }
        l.value(
            "Savings priorities",
            if data.priorities_note.is_empty() {
                "None recorded"
            } else {
                &data.priorities_note
            },
        )?;
        l.value(
            "Emergency coverage (months)",
            result.coverage_months.as_deref().unwrap_or("Unknown"),
        )?;
        l.value(
            "Primary emergency completion",
            result
                .emergency
                .completion_date
                .as_deref()
                .unwrap_or("Not projected"),
        )?;
        l.value(
            "Emergency forecast status",
            &plan_term(&result.emergency.status),
        )?;
        for (i, g) in data.goals.iter().enumerate() {
            let outcome = &result.goals[i];
            l.value("Savings goal", &g.label)?;
            l.value(
                "Target / due date",
                &format!(
                    "{} / {}",
                    optional(&g.target_amount),
                    g.target_date.as_deref().unwrap_or("Unknown")
                ),
            )?;
            l.value(
                "Reserved / monthly contribution",
                &format!(
                    "{} / {}",
                    optional(&g.reserved_amount),
                    optional(&g.monthly_contribution)
                ),
            )?;
            l.value(
                "Projected completion / status",
                &format!(
                    "{} / {}",
                    outcome
                        .forecast
                        .completion_date
                        .as_deref()
                        .unwrap_or("Not projected"),
                    plan_term(&outcome.forecast.status)
                ),
            )?;
            l.value(
                "Suggested contribution",
                &optional(&outcome.suggested_contribution),
            )?;
            l.value(
                "On track",
                match outcome.on_track {
                    Some(true) => "Yes",
                    Some(false) => "No",
                    None => "Unknown",
                },
            )?;
            for w in &outcome.warnings {
                l.text(w, 10.0)?;
            }
        }
        for w in &result.warnings {
            l.text(w, 10.0)?;
        }
        l.text("Savings uses the primary saved contribution. Alternative contribution forecasts remain comparison references and are not selected allocations.",10.0)?;
    } else {
        l.text(
            "No saved savings plan. No target, contribution or milestone is inferred.",
            10.0,
        )?;
    }
    l.section("Captured dated cash-flow assumptions / independent schedule")?;
    if let (Some(data), Some(result)) = (&c.data, &c.summary) {
        l.value(
            "Entered cash-flow period",
            &format!("{} to {}", data.start_date, data.end_date),
        )?;
        l.value(
            "Schedule notes",
            if data.notes.is_empty() {
                "None recorded"
            } else {
                &data.notes
            },
        )?;
        for a in &data.accounts {
            l.value(
                &format!("Account: {}", a.label),
                &format!(
                    "Opening {} / projected closing {}",
                    optional(&a.opening_balance),
                    optional(
                        &result
                            .closing_balances
                            .iter()
                            .find(|b| b.account_id == a.id)
                            .and_then(|b| b.amount.clone())
                    )
                ),
            )?;
        }
        for x in &data.schedules {
            l.value(
                &x.label,
                &format!(
                    "{} / {} / {} / {}",
                    plan_term(&x.kind),
                    optional(&x.amount),
                    plan_term(&x.frequency),
                    plan_term(&x.certainty)
                ),
            )?;
            l.value(
                "First / second day / until",
                &format!(
                    "{} / {} / {}",
                    x.first_date,
                    x.second_day
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "Not applicable".into()),
                    x.until_date.as_deref().unwrap_or("End of entered period")
                ),
            )?;
            l.value(
                "Source / destination account",
                &format!(
                    "{} / {}",
                    data.accounts
                        .iter()
                        .find(|a| a.id == x.account_id)
                        .map(|a| a.label.as_str())
                        .unwrap_or("Unknown"),
                    x.destination_id
                        .as_ref()
                        .and_then(|id| data.accounts.iter().find(|a| &a.id == id))
                        .map(|a| a.label.as_str())
                        .unwrap_or("None")
                ),
            )?;
        }
        l.value(
            "Projected combined closing balance",
            &optional(&result.combined_closing),
        )?;
        for w in &result.warnings {
            l.text(w, 10.0)?;
        }
        for x in result.ledger.iter().filter(|x| !x.warnings.is_empty()) {
            l.value(
                "Dated forecast concern",
                &format!("{} / {} / {}", x.date, x.label, x.warnings.join(" ")),
            )?;
        }
    } else {
        l.text(
            "No saved dated cash-flow plan. No dates or account balances are inferred.",
            10.0,
        )?;
    }
    Ok(())
}
fn chicago_time(value: &str) -> Result<String, String> {
    Ok(chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|_| "Invalid captured evidence time.")?
        .with_timezone(&chrono_tz::America::Chicago)
        .format("%Y-%m-%d %H:%M %Z")
        .to_string())
}
fn report_evidence_pdf(l: &mut Layout, p: &crate::assessments::Payload) -> Result<(), String> {
    let Some(e) = &p.supporting else {
        return Ok(());
    };
    l.section("Selected counseling session notes / counselor-authored context")?;
    l.text("Selected saved notes are counseling context, not approved financial facts. Their dates may fall outside the historical assessment period. Unselected and later notes are excluded. No note amount changes the financial calculations.",10.0)?;
    l.value(
        "Selected / available notes at capture",
        &format!("{} / {}", e.notes.len(), e.notes_count),
    )?;
    if e.notes.is_empty() {
        l.text("No session notes selected.", 10.0)?;
    }
    for n in &e.notes {
        l.value(
            "Session note (America/Chicago)",
            &format!("{} / {}", chicago_time(&n.created_at)?, n.counselor_name),
        )?;
        l.text(&n.body, 10.0)?;
        l.value("Original note ID", &n.id)?;
    }
    l.section("Captured merchant and recurring-payment review / all imported dates")?;
    l.text("Approved historical payments only, grouped across the captured client's imported dates, independently of the assessment period. Pattern suggestions remain advisory. Counselor-confirmed frequency/subscription describes reviewed historical evidence, not guaranteed future obligations or an automatically budgeted payment. Needs review is not a current confirmation.",10.0)?;
    if p.basis.merchant_groups.is_empty() {
        l.text("No current approved payment groups captured.", 10.0)?;
    }
    for g in &p.basis.merchant_groups {
        l.value(
            "Merchant / captured state",
            &format!("{} / {}", g.merchant, g.state),
        )?;
        l.value(
            "Account",
            p.basis
                .accounts
                .iter()
                .find(|a| a.id == g.account_id)
                .map(|a| a.label.as_str())
                .unwrap_or("Unknown"),
        )?;
        l.value(
            "Historical total / smallest / largest",
            &format!(
                "{} / {} / {}",
                usd(&g.total_paid),
                usd(&g.smallest_payment),
                usd(&g.largest_payment)
            ),
        )?;
        l.value(
            "Source payment dates",
            &format!(
                "{} to {} ({} payments)",
                g.evidence
                    .first()
                    .and_then(|x| x.fields.date.as_deref())
                    .unwrap_or("Unknown"),
                g.evidence
                    .last()
                    .and_then(|x| x.fields.date.as_deref())
                    .unwrap_or("Unknown"),
                g.evidence.len()
            ),
        )?;
        l.value(
            "Suggested cadence (advisory)",
            &g.suggested_frequency
                .as_ref()
                .map(plan_term)
                .unwrap_or("Not established".into()),
        )?;
        for w in &g.warnings {
            l.text(w, 10.0)?;
        }
        for x in &g.evidence {
            l.value(
                "Approved payment binding",
                &format!(
                    "Record {} / approval {} / source {} / page {} / line {}",
                    x.record_id,
                    x.approval_id,
                    x.document_id,
                    x.page
                        .map(|v| v.to_string())
                        .unwrap_or("Not applicable".into()),
                    x.line
                ),
            )?;
        }
    }
    l.section("Latest captured counselor payment decisions and original evidence")?;
    if p.basis.recurring_history.is_empty() {
        l.text(
            "No counselor payment decisions captured. Suggestions remain unreviewed.",
            10.0,
        )?;
    }
    for r in &p.basis.recurring_history {
        let state = p
            .basis
            .merchant_groups
            .iter()
            .find(|g| g.id == r.group_id)
            .map(|g| g.state.as_str())
            .unwrap_or("No current approved group; historical decision only");
        l.value(
            "Reviewed label / decision",
            &format!("{} / {}", r.label, plan_term(&r.decision)),
        )?;
        l.value("Captured applicability", state)?;
        l.value(
            "Reviewed cadence / subscription",
            &format!(
                "{} / {}",
                r.frequency
                    .as_ref()
                    .map(plan_term)
                    .unwrap_or("Not confirmed".into()),
                if r.subscription {
                    "Counselor recorded"
                } else {
                    "Not recorded"
                }
            ),
        )?;
        l.value(
            "Reviewed at (America/Chicago) / counselor",
            &format!("{} / {}", chicago_time(&r.reviewed_at)?, r.counselor_name),
        )?;
        l.value("Reason", &r.reason)?;
        l.value(
            "Decision ID / evidence digest",
            &format!("{} / {}", r.id, r.evidence_hash),
        )?;
        for x in &r.evidence {
            l.value(
                "Original reviewed payment",
                &format!(
                    "{} / {} / {} / record {} / approval {} / source {} / page {} / line {}",
                    x.fields.date.as_deref().unwrap_or("Unknown"),
                    x.fields.description,
                    optional(&x.fields.amount),
                    x.record_id,
                    x.approval_id,
                    x.document_id,
                    x.page
                        .map(|v| v.to_string())
                        .unwrap_or("Not applicable".into()),
                    x.line
                ),
            )?;
        }
    }
    Ok(())
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
    l.text(if r.needs_review{"NEEDS REVIEW: source, client, firm or saved planning context changed. Historical figures are preserved."}else{"Recorded context matches the current source, client profile and any saved plans at export."},10.0)?;
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
    l.text(if p.planning.is_some(){"Approved imported activity only. Statement coverage remains unverified. Pending, NeedsClarification, Rejected, undated and out-of-period records do not supply historical financial amounts. A separately labeled hypothetical saved planning scenario is captured below; its entered debt terms, allocations and forecasts do not change approved facts or establish a final budget agreement/client action plan. Session and recurring evidence is included only when explicitly selected, in a separately labeled section below. Counselor text is advice, not approved transaction data. Balance facts are separately dated evidence; transaction running balances do not verify assets."}else{"Approved imported activity only. Statement coverage remains unverified. Pending, NeedsClarification, Rejected, undated and out-of-period records do not supply financial amounts. Internal assessment only; no finalized client action plan, selected planning scenarios, debt APR/repayment terms or budget agreement are included. Session and recurring evidence is included only when explicitly selected below. Counselor text is advice, not approved transaction data. Balance facts are separately dated evidence; transaction running balances do not verify assets."},10.0)?;
    if p.supporting.is_some() {
        l.text("Selected saved session notes and latest counselor payment decisions are captured separately below with their original evidence. They do not change approved financial totals or verify future commitments.",10.0)?;
    }
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
        let warning = if warning == crate::dashboard::LIVE_VIEW_WARNING {
            "These figures and the assessment period were captured when the record was saved. Later decisions do not rewrite them. Review percentages measure decisions on dated imported rows, not statement coverage."
        } else {
            warning
        };
        l.text(warning, 10.0)?;
    }
    if let Some(i) = &p.intake {
        l.section("Selected intake context / counselor-entered information")?;
        l.text("Only explicitly selected intake sections are captured here. These are counselor-entered context, not verified financial facts, approved income or model assumptions. Missing information is not zero; follow-up tasks and their outcome notes are excluded.",10.0)?;
        l.value("Saved intake version", &i.revision.to_string())?;
        l.value("Intake saved at (UTC)", &i.updated_at)?;
        fn value(v: &str) -> &str {
            if v.is_empty() {
                "Not recorded"
            } else {
                v
            }
        }
        if i.sections.iter().any(|s| s == "address") {
            l.value("Mailing address", value(&i.values.address))?;
        }
        if i.sections.iter().any(|s| s == "household") {
            l.value("Household composition", value(&i.values.household))?;
        }
        if i.sections.iter().any(|s| s == "employment") {
            l.value(
                "Employment status",
                crate::report_intake::employment_label(&i.values.employment),
            )?;
            for (label, v) in [
                ("Employer", &i.values.employer),
                ("Occupation", &i.values.occupation),
                ("Employment and income context", &i.values.income_notes),
            ] {
                l.value(label, value(v))?;
            }
        }
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
    if let Some(r) = &r.scenario {
        let c = &r.payload.comparison;
        l.section("Selected saved combined scenario / hypothetical")?;
        l.value("Scenario label", &c.input.label)?;
        l.value("Scenario record", &r.payload.id)?;
        l.value("Scenario digest", &r.digest)?;
        l.value(
            "Originally recorded (America/Chicago)",
            &chicago_time(&c.created_at)?,
        )?;
        l.text("Original baseline and proposal are preserved in the encrypted scenario record. This selection does not change approved financial facts, live plans or client agreements. The planning section below uses the selected proposal, not the live baseline.",10.0)?;
        for expense in &c.input.recurring_expenses {
            l.value(
                "New proposed recurring expense",
                &format!(
                    "{} / {} / {} per {} / estimated",
                    expense.label,
                    plan_term(&expense.kind),
                    optional(&expense.amount),
                    plan_term(&expense.frequency)
                ),
            )?;
            if let Some(dates) = &expense.dated {
                let account = c
                    .proposed
                    .plans
                    .cashflow
                    .data
                    .as_ref()
                    .and_then(|p| p.accounts.iter().find(|a| a.id == dates.account_id))
                    .ok_or("The proposed expense account is unavailable.")?;
                l.value(
                    "Explicit expense payment dates",
                    &format!(
                        "{} / first {} / second day {} / through {}",
                        account.label,
                        dates.first_date,
                        dates
                            .second_day
                            .map(|d| d.to_string())
                            .unwrap_or_else(|| "Not applicable".into()),
                        dates.until_date.as_deref().unwrap_or("End of forecast")
                    ),
                )?;
            } else {
                l.value("Expense dated forecast", "Budget only; no dated bill added")?;
            }
        }
        for extra in &c.input.debt_extras {
            l.value(
                "Additional proposed debt payment",
                &format!("{} / {}", extra.month, usd(&extra.amount)),
            )?;
        }
        for change in &c.input.goal_contributions {
            let goal = &c
                .baseline
                .plans
                .savings
                .data
                .as_ref()
                .and_then(|s| s.goals.get(change.index))
                .ok_or("The captured goal baseline is unavailable.")?;
            l.value(
                "Proposed goal monthly contribution",
                &format!("{} / {}", goal.label, optional(&change.amount)),
            )?;
        }
        for (label, a, b) in [
            (
                "Entered monthly income",
                scenario_income(&c.baseline.plans),
                scenario_income(&c.proposed.plans),
            ),
            (
                "Savings model first-month buffer",
                c.baseline.first_month_buffer.as_deref(),
                c.proposed.first_month_buffer.as_deref(),
            ),
            (
                "Dated cash-flow closing",
                c.baseline
                    .plans
                    .cashflow
                    .summary
                    .as_ref()
                    .and_then(|s| s.combined_closing.as_deref()),
                c.proposed
                    .plans
                    .cashflow
                    .summary
                    .as_ref()
                    .and_then(|s| s.combined_closing.as_deref()),
            ),
        ] {
            l.value(
                label,
                &format!(
                    "Baseline {} / proposed {}",
                    a.map(usd).unwrap_or_else(|| "Unavailable".into()),
                    b.map(usd).unwrap_or_else(|| "Unavailable".into())
                ),
            )?;
        }
        for warning in &c.warnings {
            l.text(warning, 10.0)?;
        }
        if !c.income_periods.is_empty() {
            l.section("Temporary income funding and recovery")?;
            l.text("The main planning assumptions retain normal recurring income. These monthly checks apply temporary reductions against continuing debt and capped savings commitments. Negative buffers require funding or revised commitments. Payoff and goal dates remain conditional; no savings drawdown is simulated. Only explicitly selected paycheck schedules change dated receipts. Review allocations in a separate planning report before recording a client agreement.",10.0)?;
            for input in &c.input.income_reductions {
                let line = c
                    .proposed
                    .plans
                    .budget
                    .lines
                    .get(input.budget_index)
                    .ok_or("Missing temporary income basis.")?;
                l.value(
                    "Temporary income assumption",
                    &format!(
                        "{} / {} through {} / {} per {}",
                        line.label,
                        input.from_month,
                        input.through_month,
                        optional(&input.amount),
                        plan_term(&line.frequency)
                    ),
                )?;
            }
            for month in &c.income_periods {
                l.value(
                    "Income period",
                    &format!(
                        "{} / {}",
                        month.month,
                        if month.reduced {
                            "Reduced"
                        } else {
                            "Normal / recovery"
                        }
                    ),
                )?;
                l.value(
                    "Normal / period monthly income",
                    &format!(
                        "{} / {}",
                        optional(&month.normal_income),
                        optional(&month.income)
                    ),
                )?;
                l.value(
                    "Continuing debt / savings reserved",
                    &format!(
                        "{} / {}",
                        optional(&month.debt_reserved),
                        optional(&month.savings_reserved)
                    ),
                )?;
                l.value(
                    "Normal / period funding buffer",
                    &format!(
                        "{} / {}",
                        optional(&month.normal_buffer),
                        optional(&month.buffer)
                    ),
                )?;
            }
        }
    }
    if let Some(s) = &p.planning {
        planning_pdf(&mut l, s)?;
    }
    report_evidence_pdf(&mut l, p)?;
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
pub fn render_action_plan(r: &crate::action_plans::Record) -> Result<(Vec<u8>, usize), String> {
    crate::action_plans::validate(&r.payload, &r.assessment)?;
    let p = &r.payload;
    let basis = &r.assessment.payload;
    let historical = &basis.assessment;
    let scenario = basis
        .planning
        .as_ref()
        .ok_or("A captured scenario is required.")?;
    let mut l = Layout::new()?;
    l.text(&basis.workspace.organization_name, 18.0)?;
    l.text("Your financial action plan", 18.0)?;
    l.text(&p.title, 13.0)?;
    l.value(
        "Prepared for",
        &format!("{} {}", basis.client.first_name, basis.client.last_name),
    )?;
    l.value("Counselor", &basis.workspace.counselor_name)?;
    l.value("Report date (America/Chicago)", &p.report_date)?;
    l.value(
        "Historical assessment period",
        &format!("{} to {}", historical.period_start, historical.period_end),
    )?;
    l.text(if r.needs_review{"NEEDS REVIEW: supporting data changed after this plan was recorded. These original steps, allocations and forecasts are historical; review current information before acting."}else{"COUNSELOR-REVIEWED GUIDANCE / INCOMPLETE FINANCIAL BASIS"},10.0)?;
    for reason in &r.review_reasons {
        l.text(reason, 10.0)?;
    }
    l.section("Your prioritized next steps")?;
    for (i, a) in p.data.actions.iter().enumerate() {
        l.text(&format!("{}. {}", i + 1, a.step), 12.0)?;
        l.value(
            "Responsible person",
            match a.owner {
                crate::action_plans::Owner::Client => "Client",
                crate::action_plans::Owner::Counselor => "Counselor",
            },
        )?;
        l.value("Target date", a.due_date.as_deref().unwrap_or("To arrange"))?;
    }
    l.section("Agreed monthly budget and allocations")?;
    l.text(&p.data.budget_recommendations, 10.0)?;
    l.value("Captured scenario", &scenario.label)?;
    l.text("The figures below are entered monthly planning assumptions. They are separate from historical statement totals. Debt and primary savings payments follow the selected saved models when available, with budget allocations used when those models are missing.",10.0)?;
    for (label, val) in [
        ("Monthly planning income", &p.allocations.monthly_income),
        ("Necessary spending", &p.allocations.necessary),
        ("Discretionary spending", &p.allocations.discretionary),
        ("Surplus before allocations", &p.allocations.surplus),
        ("Agreed debt allocation", &p.allocations.debt),
        ("Agreed primary savings allocation", &p.allocations.savings),
        ("Unallocated / buffer", &p.allocations.unallocated),
    ] {
        l.value(label, &usd(val))?;
    }
    l.section("Historical financial position / approved imported activity")?;
    for (label, val) in [
        ("Approved income receipts", &historical.totals.income),
        ("Gross spending", &historical.totals.spending),
        ("Expense refunds", &historical.totals.refunds),
        ("Net spending", &historical.totals.net_spending),
        ("Operating cash flow", &historical.operating_cashflow),
    ] {
        l.value(label, &usd(val))?;
    }
    let f = &historical.facts_metrics;
    l.value(
        "Verified inventory date",
        f.as_of.as_deref().unwrap_or("Unknown"),
    )?;
    for (label, val) in [
        ("Recorded cash", &f.cash),
        ("Recorded accessible savings", &f.accessible_savings),
        ("Recorded debt", &f.debt),
        ("Estimated net worth at inventory date", &f.net_worth),
    ] {
        l.value(label, &optional(val))?;
    }
    if !historical.categories.is_empty() {
        l.section("Approved spending breakdown")?;
        for c in &historical.categories {
            l.bar(&c.label, &usd(&c.spending), c.scale)?;
        }
    }
    l.section("Selected debt strategy and conditional milestones")?;
    l.value(
        "Strategy",
        match scenario.debt_strategy {
            crate::planning::Strategy::None => "No repayment projection selected",
            crate::planning::Strategy::Snowball => "Snowball",
            crate::planning::Strategy::Avalanche => "Avalanche",
        },
    )?;
    if let Some(result) = crate::planning::projection(scenario) {
        l.value("Projected outcome", &plan_term(&result.status))?;
        l.value(
            "Projected payoff month",
            result.payoff_month.as_deref().unwrap_or("Not projected"),
        )?;
        l.value("Projected total interest", &usd(&result.total_interest))?;
        l.value("Projected fees", &usd(&result.total_fees))?;
        l.value(
            "Projected remaining balance",
            &usd(&result.remaining_balance),
        )?;
        if let Some(d) = &scenario.plans.debt.data {
            for (i, a) in d.accounts.iter().enumerate() {
                l.value(
                    &format!("Projected payoff: {}", a.label),
                    result.account_payoff_months[i]
                        .as_deref()
                        .unwrap_or("Not projected"),
                )?;
            }
        }
    } else {
        l.text(
            "No payoff milestone is promised. Review the missing or unsupported repayment terms.",
            10.0,
        )?;
    }
    l.section("Emergency savings and goal milestones")?;
    if let Some(v) = &scenario.plans.savings.summary {
        l.value("Emergency target", &optional(&v.emergency_target))?;
        l.value(
            "Accessible emergency pool (entered plan)",
            &optional(&v.accessible_emergency),
        )?;
        l.value(
            "Primary emergency contribution",
            &optional(&v.emergency.contribution),
        )?;
        l.value(
            "Projected emergency completion",
            v.emergency
                .completion_date
                .as_deref()
                .unwrap_or("Not projected"),
        )?;
        l.value("Emergency forecast status", &plan_term(&v.emergency.status))?;
        if let Some(d) = &scenario.plans.savings.data {
            for (i, g) in d.goals.iter().enumerate() {
                l.value(
                    &g.label,
                    &format!(
                        "Target {} / due {} / entered monthly {} / projected completion {}",
                        optional(&g.target_amount),
                        g.target_date.as_deref().unwrap_or("Unknown"),
                        optional(&g.monthly_contribution),
                        v.goals[i]
                            .forecast
                            .completion_date
                            .as_deref()
                            .unwrap_or("Not projected")
                    ),
                )?;
            }
        }
    } else {
        l.text(
            "No savings forecast is captured. Verify goals and deadlines at follow-up.",
            10.0,
        )?;
    }
    l.section("Follow-up arrangements")?;
    l.text(&p.data.follow_up, 11.0)?;
    l.value(
        "Follow-up date",
        p.data.follow_up_date.as_deref().unwrap_or("To arrange"),
    )?;
    l.section("Reviewed limitations and assumptions")?;
    l.text(&p.data.limitations, 10.0)?;
    l.value(
        "Captured scenario assumptions",
        if scenario.assumptions.is_empty() {
            "None recorded"
        } else {
            &scenario.assumptions
        },
    )?;
    for warning in &p.disclosures {
        l.text(warning, 9.0)?;
    }
    l.text("Keep this local PDF in a private location; it contains readable client information outside workspace encryption. Original statements and internal transaction evidence are retained separately, not attached. This plan is recorded by the counselor and is not an electronic client signature or professional certification.",9.0)?;
    l.value("Supporting assessment", &p.assessment_id)?;
    l.value("Action plan record", &p.id)?;
    l.finish_labeled(
        if r.needs_review {
            "CLIENT ACTION PLAN / NEEDS REVIEW"
        } else {
            "CLIENT ACTION PLAN / INCOMPLETE BASIS"
        },
        "Client action plan - reviewed guidance",
    )
}
pub fn save(r: &Record, path: &str, workspace: &Path) -> Result<Export, String> {
    let (bytes, pages) = render(r, &chrono::Utc::now().to_rfc3339())?;
    save_bytes(&r.payload.id, bytes, pages, path, workspace)
}
pub fn save_bytes(
    id: &str,
    bytes: Vec<u8>,
    pages: usize,
    path: &str,
    workspace: &Path,
) -> Result<Export, String> {
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
        record_id: id.to_owned(),
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
