//! Offline, bounded statement-row detection. Selection is never approval.
use crate::{
    budget::money,
    imports::{self, DateFormat, Fields, Kind, Raw},
    pdf::{self, Locator, Page},
};
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Account {
    Bank,
    Card,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Layout {
    Auto,
    Signed,
    AmountBalance,
    BalanceAmount,
    DebitCredit,
    DebitCreditBalance,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub account: Account,
    pub date_format: Option<DateFormat>,
    pub layout: Layout,
    pub invert: bool,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            account: Account::Bank,
            date_format: None,
            layout: Layout::Auto,
            invert: false,
            period_start: None,
            period_end: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Evidence {
    pub version: u32,
    pub config: Config,
    pub selected: Vec<Locator>,
    pub pages: Vec<Page>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Choice {
    pub locator: Locator,
    pub checked: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub token: String,
    pub config: Config,
    pub choices: Vec<Choice>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub locator: Locator,
    pub through_line: u32,
    pub text: String,
    pub date: Option<String>,
    pub description: String,
    pub amount: Option<String>,
    pub balance: Option<String>,
    pub clear: bool,
    pub explanation: String,
    #[serde(skip)]
    raw: Option<Raw>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub count: usize,
    pub outflows: String,
    pub inflows: String,
    pub net: String,
    pub unknown_amounts: usize,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    pub client_id: String,
    pub token: String,
    pub candidates: Vec<Candidate>,
    pub selected: Vec<Locator>,
    pub summary: Summary,
    pub page_count: usize,
    pub transaction_pages: usize,
    pub excluded_lines: usize,
    pub warnings: Vec<String>,
    pub detected_start: Option<String>,
    pub detected_end: Option<String>,
}
#[derive(Clone, Copy, Default)]
struct Context {
    direction: i8,
    balance: bool,
    debit_credit: bool,
}
fn lower(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
fn context_line(s: &str) -> bool {
    if boilerplate(s) || s.len() > 180 {
        return false;
    }
    let s = lower(s);
    if s.split_whitespace().any(|w| {
        matches!(
            w,
            "you" | "your" | "may" | "will" | "please" | "must" | "contact"
        )
    }) {
        return false;
    }
    !s.chars().next().is_some_and(|c| c.is_ascii_digit())
        && (s.contains("statement period")
            || s.contains("statement from")
            || s.contains("billing period")
            || s.contains("deposits and")
            || s == "deposits"
            || s == "credits"
            || s.contains("payments and credits")
            || s.contains("withdrawals")
            || s.contains("electronic payments")
            || s.contains("debits and")
            || s.contains("purchases")
            || s.contains("transaction detail")
            || s.contains("transaction history")
            || s.contains("account activity")
            || s == "fees"
            || s == "interest charged"
            || (s.contains("date") && (s.contains("description") || s.contains("transaction"))))
}
fn update_context(line: &str, c: &mut Context, account: &Account) {
    let s = lower(line);
    if !context_line(line) {
        return;
    }
    if s.contains("date") && (s.contains("description") || s.contains("transaction")) {
        c.balance = s.contains("balance");
        c.debit_credit = (s.contains("debit") || s.contains("withdrawal"))
            && (s.contains("credit") || s.contains("deposit"));
        return;
    }
    if s.contains("withdrawals") || s.contains("electronic payments") || s.contains("debits and") {
        c.direction = if *account == Account::Bank { -1 } else { 1 };
    } else if s.contains("deposits") || s == "credits" || s.contains("payments and credits") {
        c.direction = if *account == Account::Bank { 1 } else { -1 };
    } else if s.contains("purchases") || s == "fees" || s == "interest charged" {
        c.direction = if *account == Account::Card { 1 } else { -1 };
    } else if s.contains("transaction detail")
        || s.contains("transaction history")
        || s.contains("account activity")
    {
        c.direction = 0;
    }
}
fn boilerplate(s: &str) -> bool {
    let s = lower(s);
    [
        "questions?",
        "questions please",
        "please contact",
        "for questions",
        "contact us",
        "member fdic",
        "equal housing",
        "important notice",
        "important information",
        "privacy notice",
        "billing rights",
        "how to avoid",
        "your rights",
        "payment due",
        "minimum payment",
        "credit limit",
        "available credit",
        "annual percentage",
        "interest charge calculation",
        "account summary",
        "balance summary",
        "beginning balance",
        "opening balance",
        "closing balance",
        "ending balance",
        "previous balance",
        "balance forward",
        "balance brought forward",
        "statement balance",
        "daily balance",
        "total deposits",
        "total withdrawals",
        "total payments",
        "total purchases",
        "total fees",
        "total service fees",
        "total interest",
        "total credits",
        "total debits",
        "total for this period",
        "total for this statement",
        "subtotal",
        "page ",
        "continued on",
        "customer service",
        "www.",
        "http:",
        "https:",
    ]
    .iter()
    .any(|p| s.starts_with(p))
        || matches!(s.as_str(), "total" | "totals" | "balance")
        || s.contains("please contact us")
        || s.contains("call us at")
        || s.contains("annual percentage rate")
        || s.contains("fees may apply")
        || s.contains("terms and conditions")
}
fn date_token(token: &str) -> bool {
    let t = token.trim_matches(|c: char| c == ',' || c == ':' || c == '*');
    let parts = t.split(['/', '-']).collect::<Vec<_>>();
    (parts.len() == 2 || parts.len() == 3)
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 4 && p.bytes().all(|b| b.is_ascii_digit()))
}
fn complete_date(token: &str, format: &Option<DateFormat>) -> Option<String> {
    let t = token.trim_matches(|c: char| c == ',' || c == ':' || c == '*');
    // ISO is unambiguous even when the transaction table uses a different style.
    if let Some(iso) = imports::source_date(t, &DateFormat::Iso) {
        return Some(iso);
    }
    if let Some(f) = format {
        return imports::source_date(t, f);
    }
    let mut dates = BTreeSet::new();
    for f in [DateFormat::Iso, DateFormat::Mdy, DateFormat::Dmy] {
        if let Some(d) = imports::source_date(t, &f) {
            dates.insert(d);
        }
    }
    if dates.len() == 1 {
        dates.pop_first()
    } else {
        None
    }
}
fn resolved_config(pages: &[Page], config: &Config) -> Config {
    let mut c = config.clone();
    if c.date_format.is_none() {
        let (mut mdy, mut dmy) = (false, false);
        for token in pages
            .iter()
            .flat_map(|p| p.text.lines())
            .filter(|line| {
                let l = lower(line);
                let mut words = line.split_whitespace();
                let first = words.next().unwrap_or_default();
                date_token(first)
                    || (first.bytes().all(|b| b.is_ascii_digit())
                        && words.next().is_some_and(date_token))
                    || (context_line(line)
                        && (l.contains("statement period")
                            || l.contains("billing period")
                            || l.contains("statement from")))
            })
            .flat_map(str::split_whitespace)
            .filter(|t| date_token(t))
        {
            let t = token.trim_matches(|c: char| c == ',' || c == ':' || c == '*');
            if t.split('/').count() != 3 {
                continue;
            }
            let m = imports::source_date(t, &DateFormat::Mdy).is_some();
            let d = imports::source_date(t, &DateFormat::Dmy).is_some();
            mdy |= m && !d;
            dmy |= d && !m;
        }
        if mdy && !dmy {
            c.date_format = Some(DateFormat::Mdy);
        } else if dmy && !mdy {
            c.date_format = Some(DateFormat::Dmy);
        }
    }
    c
}
fn detected_period(
    pages: &[Page],
    format: &Option<DateFormat>,
) -> (Option<String>, Option<String>) {
    let mut pairs = BTreeSet::new();
    for line in pages.iter().flat_map(|p| p.text.lines()).filter(|s| {
        let s = lower(s);
        s.contains("statement period")
            || s.contains("billing period")
            || s.contains("statement from")
    }) {
        let dates = line
            .split_whitespace()
            .filter(|t| date_token(t))
            .filter_map(|t| complete_date(t, format))
            .collect::<Vec<_>>();
        if dates.len() == 2 && dates[0] <= dates[1] {
            if let (Ok(a), Ok(b)) = (imports::iso_date(&dates[0]), imports::iso_date(&dates[1])) {
                if (b - a).num_days() <= 366 {
                    pairs.insert((dates[0].clone(), dates[1].clone()));
                }
            }
        }
    }
    if pairs.len() == 1 {
        let (a, b) = pairs.pop_first().unwrap();
        (Some(a), Some(b))
    } else {
        (None, None)
    }
}
fn resolve_date(
    token: &str,
    c: &Config,
    period: &(Option<String>, Option<String>),
) -> Option<String> {
    if let Some(d) = complete_date(token, &c.date_format) {
        return Some(d);
    }
    let parts = token.split('/').collect::<Vec<_>>();
    if parts.len() != 2 {
        return None;
    }
    let a = imports::iso_date(c.period_start.as_deref().or(period.0.as_deref())?).ok()?;
    let b = imports::iso_date(c.period_end.as_deref().or(period.1.as_deref())?).ok()?;
    if a > b || (b - a).num_days() > 366 {
        return None;
    }
    let (x, y) = (parts[0].parse::<u32>().ok()?, parts[1].parse::<u32>().ok()?);
    let mut dates = BTreeSet::new();
    for year in a.year()..=b.year() {
        let possibilities = match c.date_format {
            Some(DateFormat::Dmy) => vec![(y, x)],
            Some(DateFormat::Mdy) | Some(DateFormat::Iso) => vec![(x, y)],
            None => vec![(x, y), (y, x)],
        };
        for (month, day) in possibilities {
            if let Some(d) = NaiveDate::from_ymd_opt(year, month, day) {
                if d >= a && d <= b {
                    dates.insert(d.to_string());
                }
            }
        }
    }
    if dates.len() == 1 {
        dates.pop_first()
    } else {
        None
    }
}
#[derive(Clone)]
struct Amount {
    start: usize,
    end: usize,
    raw: String,
    cents: i64,
    explicit: bool,
}
fn amount_token(value: &str, account: &Account) -> Option<(i64, bool)> {
    let t = value.replace('−', "-");
    let mut t = t.trim().to_owned();
    let mut direction = 0;
    if t.to_ascii_uppercase().ends_with("CR") {
        t.truncate(t.len() - 2);
        direction = if *account == Account::Bank { 1 } else { -1 };
    } else if t.to_ascii_uppercase().ends_with("DR") {
        t.truncate(t.len() - 2);
        direction = if *account == Account::Bank { -1 } else { 1 };
    }
    if t.ends_with('-') {
        t.pop();
        direction = -1;
    }
    let decimal = t
        .trim_end_matches(')')
        .rsplit('.')
        .next()
        .is_some_and(|v| v.len() == 2 && v.bytes().all(|b| b.is_ascii_digit()))
        && t.contains('.');
    if !decimal && !t.contains('$') {
        return None;
    }
    let explicit = direction != 0 || t.starts_with(['-', '+', '(']);
    let mut n = imports::reported_money(&t).ok()?;
    if direction != 0 {
        if n < 0 && direction > 0 {
            return None;
        }
        n = n.abs() * i64::from(direction);
    }
    Some((n, explicit))
}
fn amounts(words: &[&str], account: &Account) -> Vec<Amount> {
    let mut out = vec![];
    let mut i = 0;
    while i < words.len() {
        let start = i;
        let mut raw = words[i].to_owned();
        if raw == "$" && i + 1 < words.len() {
            i += 1;
            raw.push_str(words[i]);
        }
        if i + 1 < words.len() && ["CR", "DR"].contains(&words[i + 1].to_ascii_uppercase().as_str())
        {
            i += 1;
            raw.push_str(words[i]);
        }
        if let Some((cents, explicit)) = amount_token(&raw, account) {
            out.push(Amount {
                start,
                end: i + 1,
                raw,
                cents,
                explicit,
            });
        }
        i += 1;
    }
    out
}
fn validate_config(c: &Config) -> Result<(), String> {
    for s in [&c.period_start, &c.period_end].into_iter().flatten() {
        imports::iso_date(s)?;
    }
    if let (Some(a), Some(b)) = (&c.period_start, &c.period_end) {
        let (a, b) = (imports::iso_date(a)?, imports::iso_date(b)?);
        if a > b || (b - a).num_days() > 366 {
            return Err("Use an ordered statement period of at most 367 days.".into());
        }
    }
    Ok(())
}
fn detect(pages: &[Page], config: &Config) -> Result<Vec<Candidate>, String> {
    pdf::validate_pages(pages)?;
    validate_config(config)?;
    let resolved = resolved_config(pages, config);
    let config = &resolved;
    let period = detected_period(pages, &config.date_format);
    let mut output = vec![];
    let mut context = Context::default();
    for p in pages {
        // Column headings do not carry across pages; a named debit/credit section may.
        context.balance = false;
        context.debit_credit = false;
        let lines = p.text.lines().collect::<Vec<_>>();
        let mut i = 0;
        while i < lines.len() {
            let first = lines[i].trim();
            update_context(first, &mut context, &config.account);
            if first.is_empty() || context_line(first) || boilerplate(first) {
                i += 1;
                continue;
            }
            let start = i;
            let mut text = first.to_owned();
            let mut words = text.split_whitespace().collect::<Vec<_>>();
            let date_index = if words.first().is_some_and(|s| date_token(s)) {
                Some(0)
            } else if words.len() > 1
                && words[0].bytes().all(|b| b.is_ascii_digit())
                && date_token(words[1])
            {
                Some(1)
            } else {
                None
            };
            let mut values = amounts(&words, &config.account);
            if date_index.is_some() && values.is_empty() {
                while i + 1 < lines.len() && i - start < 2 {
                    let next = lines[i + 1].trim();
                    if next.is_empty()
                        || boilerplate(next)
                        || context_line(next)
                        || next.split_whitespace().next().is_some_and(date_token)
                    {
                        break;
                    }
                    text.push('\n');
                    text.push_str(next);
                    i += 1;
                    words = text.split_whitespace().collect();
                    values = amounts(&words, &config.account);
                    if !values.is_empty() {
                        break;
                    }
                }
            }
            let mut desc_start = date_index.map_or(0, |n| n + 1);
            if desc_start < words.len() && date_token(words[desc_start]) {
                desc_start += 1;
            }
            let first_amount = values.first().map_or(words.len(), |a| a.start);
            let desc = words.get(desc_start..first_amount).unwrap_or(&[]).join(" ");
            // Standalone undated text is not transaction evidence. In particular,
            // a disclosure ending in money must never become a selectable row.
            if date_index.is_none() || boilerplate(&desc) || desc.is_empty() {
                i += 1;
                continue;
            }
            if desc.chars().count() > 500 {
                i += 1;
                continue;
            }
            let ends_money = values.last().is_some_and(|a| a.end == words.len());
            let layout = match config.layout {
                Layout::Auto if context.debit_credit => {
                    if context.balance {
                        Layout::DebitCreditBalance
                    } else {
                        Layout::DebitCredit
                    }
                }
                Layout::Auto if context.balance => Layout::AmountBalance,
                Layout::Auto => Layout::Signed,
                _ => config.layout.clone(),
            };
            let mut chosen = None;
            let mut balance = None;
            let (mut debit, mut credit) = (None, None);
            let mut issue = "";
            if ends_money {
                match layout {
                    Layout::Signed|Layout::Auto if values.len()==1=>chosen=values.first().cloned(),
                    Layout::AmountBalance if values.len()==2=>{chosen=Some(values[0].clone());balance=Some(values[1].clone());},
                    Layout::BalanceAmount if values.len()==2=>{balance=Some(values[0].clone());chosen=Some(values[1].clone());},
                    Layout::DebitCredit|Layout::DebitCreditBalance=>{
                        let expected=if layout==Layout::DebitCreditBalance{3}else{2};
                        if values.len()==expected && values[0].cents>=0&&values[1].cents>=0&&(values[0].cents==0||values[1].cents==0) {
                            let n=if config.account==Account::Bank{values[1].cents-values[0].cents}else{values[0].cents-values[1].cents};
                            let mut a=values[if values[0].cents!=0{0}else{1}].clone();a.cents=n;a.explicit=true;chosen=Some(a);debit=Some(values[0].raw.clone());credit=Some(values[1].raw.clone());if expected==3{balance=Some(values[2].clone());}
                        } else {issue="Separate debit/credit cells are ambiguous or missing. Verify the amount in review; blank columns cannot be reconstructed safely from flattened text.";}
                    },
                    _=>issue="Multiple amounts or an unsupported column order. Choose the actual amount layout or check this row for individual correction.",
                }
            }
            let n = chosen.as_ref().map(|a| {
                let mut n = a.cents;
                if !a.explicit && context.direction != 0 {
                    n = n.abs() * i64::from(context.direction);
                }
                if config.invert {
                    -n
                } else {
                    n
                }
            });
            let date = date_index.and_then(|d| resolve_date(words[d], config, &period));
            let outside_period = date.as_ref().is_some_and(|d| {
                config
                    .period_start
                    .as_ref()
                    .or(period.0.as_ref())
                    .is_some_and(|s| d < s)
                    || config
                        .period_end
                        .as_ref()
                        .or(period.1.as_ref())
                        .is_some_and(|s| d > s)
            });
            let clear = !outside_period
                && date.is_some()
                && n.is_some()
                && desc.chars().any(char::is_alphabetic);
            let explanation = if outside_period {
                "The transaction date falls outside the statement period. Verify its source and date before selecting it.".into()
            } else if !issue.is_empty() {
                issue.to_owned()
            } else if date.is_none() {
                "Date needs verification. Choose its date format and statement period for yearless dates; ambiguous dates are not guessed.".into()
            } else if n.is_none() {
                "Amount is incomplete or ambiguous; verify this transaction individually.".into()
            } else if i > start {
                "Recognizable date and amount across wrapped transaction text. Verify the original statement.".into()
            } else {
                "Recognizable transaction date, description and amount. Selection is not approval."
                    .into()
            };
            let raw=Raw{page:Some(p.number),reported_line:Some(text.clone()),extraction_method:"pdf-statement-rows-v2".into(),record_number:0,line:(start+1)as u64,reported_date:date_index.map_or(String::new(),|d|words[d].into()),reported_amount:chosen.as_ref().map(|a|a.raw.clone()),reported_debit:debit,reported_credit:credit,reported_description:desc.clone(),reported_merchant:None,reported_category:None,reported_kind:None,reported_reference:None,reported_balance:balance.as_ref().map(|a|a.raw.clone()),extracted:Fields{date:date.clone(),amount:n.map(money),description:desc.clone(),merchant:String::new(),category:String::new(),kind:Kind::Unclassified},reference:None,balance:balance.as_ref().map(|a|money(a.cents)),issues:vec!["PDF transaction candidate: verify original date, amount, sign, category and transaction kind before approval. Selection and detection do not verify a financial institution.".into(),explanation.clone()]};
            output.push(Candidate {
                locator: Locator {
                    page: p.number,
                    line: (start + 1) as u32,
                },
                through_line: (i + 1) as u32,
                text,
                date,
                description: desc,
                amount: n.map(money),
                balance: raw.balance.clone(),
                clear,
                explanation,
                raw: Some(raw),
            });
            i += 1;
        }
    }
    Ok(output)
}
fn key(l: &Locator) -> (u32, u32) {
    (l.page, l.line)
}
pub fn inspect(
    client_id: String,
    token: String,
    pages: &[Page],
    config: Config,
    choices: Vec<Choice>,
) -> Result<Inspection, String> {
    if choices.len() > pdf::MAX_SELECTED * 4 {
        return Err("Too many selection overrides. Inspect the source again.".into());
    }
    let candidates = detect(pages, &config)?;
    let limit = if config.account == Account::Card {
        crate::cards::MAX_ROWS
    } else {
        pdf::MAX_SELECTED
    };
    let overrides = choices
        .iter()
        .map(|c| (key(&c.locator), c.checked))
        .collect::<BTreeMap<_, _>>();
    if overrides.len() != choices.len() {
        return Err("Choose each transaction at most once.".into());
    }
    let available = candidates
        .iter()
        .map(|c| key(&c.locator))
        .collect::<BTreeSet<_>>();
    // Date/layout edits can remove earlier candidates; their overrides do not select noise.
    let mut warnings = vec![];
    if overrides.keys().any(|k| !available.contains(k)) {
        warnings.push(
            "Some earlier row choices no longer match this layout. Review the remaining selection."
                .into(),
        );
    }
    let mut selected = vec![];
    let (mut inflows, mut outflows, mut unknown) = (0i64, 0i64, 0usize);
    for c in &candidates {
        let manual = overrides.get(&key(&c.locator)).copied();
        let checked = manual.unwrap_or(c.clear && selected.len() < limit);
        if !checked {
            continue;
        }
        if selected.len() >= limit {
            return Err(format!(
                "Select at most {limit} transactions for this source."
            ));
        }
        selected.push(c.locator.clone());
        if let Some(n) = c.amount.as_deref().map(imports::signed_money).transpose()? {
            if n < 0 {
                outflows = outflows
                    .checked_add(-n)
                    .ok_or("Selected amounts exceed range.")?;
            } else {
                inflows = inflows
                    .checked_add(n)
                    .ok_or("Selected amounts exceed range.")?;
            }
        } else {
            unknown += 1;
        }
    }
    if candidates.iter().filter(|r| r.clear).count() > limit {
        warnings.push(format!("Only the first {limit} clear candidates start selected. This selection is not a complete statement; split or revise the source before relying on it."));
    }
    warnings.push("Selected totals are an unsaved draft. Import updates the review queue; only explicitly approved and categorized facts update official analysis. Statement balances and planning inputs remain separate.".into());
    let page_count = pages.len();
    let transaction_pages = candidates
        .iter()
        .map(|r| r.locator.page)
        .collect::<BTreeSet<_>>()
        .len();
    let used = candidates
        .iter()
        .map(|r| (r.through_line - r.locator.line + 1) as usize)
        .sum::<usize>();
    let excluded_lines = pages
        .iter()
        .map(|p| p.text.lines().filter(|s| !s.trim().is_empty()).count())
        .sum::<usize>()
        .saturating_sub(used);
    let (detected_start, detected_end) =
        detected_period(pages, &resolved_config(pages, &config).date_format);
    Ok(Inspection {
        client_id,
        token,
        candidates,
        summary: Summary {
            count: selected.len(),
            outflows: money(outflows),
            inflows: money(inflows),
            net: money(inflows - outflows),
            unknown_amounts: unknown,
        },
        selected,
        page_count,
        transaction_pages,
        excluded_lines,
        warnings,
        detected_start,
        detected_end,
    })
}
pub fn capture(
    pages: &[Page],
    config: Config,
    selected: Vec<Locator>,
) -> Result<(Evidence, Vec<Raw>), String> {
    let config = resolved_config(pages, &config);
    let candidates = detect(pages, &config)?;
    let wanted = selected.iter().map(key).collect::<BTreeSet<_>>();
    let limit = if config.account == Account::Card {
        crate::cards::MAX_ROWS
    } else {
        pdf::MAX_SELECTED
    };
    if wanted.is_empty() || wanted.len() != selected.len() || wanted.len() > limit {
        return Err(format!("Select 1–{limit} distinct transaction candidates."));
    }
    let mut keep = BTreeSet::new();
    let mut rows = vec![];
    for c in candidates {
        if wanted.contains(&key(&c.locator)) {
            for n in c.locator.line..=c.through_line {
                keep.insert((c.locator.page, n));
            }
            let mut raw = c.raw.ok_or("Missing original row.")?;
            raw.record_number = rows.len() as u32 + 1;
            rows.push(raw);
        }
    }
    if rows.len() != wanted.len() {
        return Err("A selected line is not a transaction candidate in this layout. Review the inspection again.".into());
    }
    let minimal = pages
        .iter()
        .map(|p| Page {
            number: p.number,
            text: p
                .text
                .lines()
                .enumerate()
                .map(|(i, s)| {
                    if keep.contains(&(p.number, (i + 1) as u32)) || context_line(s) {
                        s
                    } else {
                        ""
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
                .trim_end_matches('\n')
                .to_owned(),
        })
        .collect();
    let mut selected = selected;
    selected.sort_by_key(key);
    let evidence = Evidence {
        version: 2,
        config,
        selected,
        pages: minimal,
    };
    Ok((evidence, rows))
}
pub fn replay(e: &Evidence) -> Result<Vec<Raw>, String> {
    if e.version != 2 {
        return Err("Unsupported statement extraction evidence.".into());
    }
    let (canonical, rows) = capture(&e.pages, e.config.clone(), e.selected.clone())?;
    if serde_json::to_vec(&canonical).map_err(|_| "Invalid PDF evidence.")?
        != serde_json::to_vec(e).map_err(|_| "Invalid PDF evidence.")?
    {
        return Err("PDF selection evidence is not canonical.".into());
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pages(rows: &[&str]) -> Vec<Page> {
        rows.iter()
            .enumerate()
            .map(|(i, s)| Page {
                number: i as u32 + 1,
                text: (*s).into(),
            })
            .collect()
    }
    fn analyze(p: &[Page], c: Config, choices: Vec<Choice>) -> Inspection {
        inspect(
            "fictional-client".into(),
            "captured-token".into(),
            p,
            c,
            choices,
        )
        .unwrap()
    }
    #[test]
    fn fine_print_cannot_supply_transaction_direction_and_out_of_period_dates_need_review() {
        let p=pages(&["Your purchases may incur a fee of $25.00\nTransaction details\nService provided by our institution $99.00\nTotal 99.00\nStatement period 2026-09-01 to 2026-09-30\nDate Description Amount Balance\n2026-09-02 Fictional wages 1000.01 1000.01\n2026-10-02 Fictional later purchase -25.11 974.90\nQuestions? Please contact us"]);
        let inspected = inspect(
            "client".into(),
            "token".into(),
            &p,
            Config::default(),
            vec![],
        )
        .unwrap();
        assert_eq!(inspected.candidates.len(), 2);
        assert_eq!(inspected.selected.len(), 1);
        assert_eq!(inspected.summary.inflows, "1000.01");
        assert_eq!(inspected.summary.outflows, "0.00");
        assert!(!inspected.candidates[1].clear);
        let (evidence, rows) = capture(&p, Config::default(), inspected.selected).unwrap();
        assert!(!evidence.pages[0].text.contains("may incur"));
        let p=pages(&["Questions? Please contact us before 12/31/2026\n03/04/2026 Fictional ambiguous purchase -25.11"]);
        let a = inspect(
            "client".into(),
            "token".into(),
            &p,
            Config::default(),
            vec![],
        )
        .unwrap();
        assert!(a.selected.is_empty());
        assert_eq!(a.candidates.len(), 1);
        let p=pages(&["Statement period 2026-09-01 to 2026-09-30\nPurchases\n09/02 Fictional card purchase 25.11"]);
        let config = Config {
            account: Account::Card,
            date_format: Some(DateFormat::Mdy),
            ..Config::default()
        };
        let a = inspect("client".into(), "token".into(), &p, config, vec![]).unwrap();
        assert_eq!(a.selected.len(), 1);
        assert_eq!(a.candidates[0].date.as_deref(), Some("2026-09-02"));
        assert_eq!(replay(&evidence).unwrap(), rows);
    }

    #[test]
    fn bank_transactions_exclude_boilerplate_balances_and_pages_and_recalculate_selection() {
        let p=pages(&["Fictional bank\nStatement period 2026-09-01 to 2026-09-30\nQuestions? Please contact us\nOpening balance $500.00\nMinimum payment due $20.00", "Deposits and additions\nDate Description Amount Balance\n09/01 Fictional payroll 1000.01 1500.01\nWithdrawals and debits\n09/02 Fictional market 25.11 1474.90\n09/03 TOTAL WINE 10.02 1464.88\n09/04 Opening balance 500.00\nTotal withdrawals 35.13", "Please contact us at 800-555-0100\nMember FDIC\nAnnual percentage rate 29.99%\nFees may apply $10.00"]);
        let a = analyze(&p, Config::default(), vec![]);
        assert_eq!(a.candidates.len(), 3);
        assert_eq!(a.selected.len(), 3);
        assert_eq!(a.transaction_pages, 1);
        assert_eq!(a.page_count, 3);
        assert_eq!(a.summary.inflows, "1000.01");
        assert_eq!(a.summary.outflows, "35.13");
        assert_eq!(a.summary.net, "964.88");
        assert_eq!(a.candidates[1].date.as_deref(), Some("2026-09-02"));
        assert_eq!(a.candidates[1].balance.as_deref(), Some("1474.90"));
        let b = analyze(
            &p,
            Config::default(),
            vec![Choice {
                locator: a.candidates[1].locator.clone(),
                checked: false,
            }],
        );
        assert_eq!(b.summary.count, 2);
        assert_eq!(b.summary.outflows, "10.02");
        assert_eq!(b.summary.net, "989.99");
        let (e, raw) = capture(&p, Config::default(), b.selected).unwrap();
        assert_eq!(raw[1].reported_amount.as_deref(), Some("10.02"));
        assert_eq!(raw[1].reported_balance.as_deref(), Some("1464.88"));
        assert!(!serde_json::to_string(&e)
            .unwrap()
            .contains("Please contact"));
        assert_eq!(replay(&e).unwrap(), raw);
        assert_eq!(raw[1].page, Some(2));
        assert_eq!(raw[1].line, 6);
    }
    #[test]
    fn card_year_rollover_wrapped_purchases_and_payment_credit_markers_preserve_originals() {
        let p=pages(&["Statement period 12/15/2026 to 01/15/2027\nPurchases\n12/31 Fictional merchant\ncontinued location $25.15\nPayments and credits\n01/02 AUTOMATIC PAYMENT 50.00CR\nQuestions? Please contact us"]);
        let c = Config {
            account: Account::Card,
            ..Config::default()
        };
        let a = analyze(&p, c.clone(), vec![]);
        assert_eq!(a.selected.len(), 2);
        assert_eq!(a.candidates[0].date.as_deref(), Some("2026-12-31"));
        assert_eq!(a.candidates[1].date.as_deref(), Some("2027-01-02"));
        assert_eq!(a.summary.inflows, "25.15");
        assert_eq!(a.summary.outflows, "50.00");
        assert_eq!(a.summary.net, "-24.85");
        let (e, rows) = capture(&p, c, a.selected).unwrap();
        assert_eq!(
            rows[0].reported_line.as_deref(),
            Some("12/31 Fictional merchant\ncontinued location $25.15")
        );
        assert_eq!(rows[1].reported_amount.as_deref(), Some("50.00CR"));
        assert_eq!(rows[1].extracted.amount.as_deref(), Some("-50.00"));
        assert_eq!(replay(&e).unwrap(), rows);
    }
    #[test]
    fn uncertain_dates_columns_and_amounts_are_not_prechecked_and_manual_choices_are_drafts() {
        let p=pages(&["2026-09-01 Merchant 10.00 990.00\n03/04/2026 Ambiguous date 5.00\n2026-09-02 Missing amount\nQuestions? Please contact us"]);
        let a = analyze(&p, Config::default(), vec![]);
        assert_eq!(a.candidates.len(), 3);
        assert!(a.selected.is_empty());
        assert_eq!(a.summary.count, 0);
        let b = analyze(
            &p,
            Config::default(),
            vec![Choice {
                locator: a.candidates[0].locator.clone(),
                checked: true,
            }],
        );
        assert_eq!(b.summary.unknown_amounts, 1);
        assert_eq!(b.summary.net, "0.00");
        let c = Config {
            layout: Layout::AmountBalance,
            ..Config::default()
        };
        let b = analyze(&p, c, vec![]);
        assert_eq!(b.selected.len(), 1);
        assert_eq!(b.summary.inflows, "10.00");
        assert!(capture(&p, Config::default(), vec![Locator { page: 1, line: 4 }]).is_err());
    }
    #[test]
    fn debit_credit_columns_fees_interest_signs_and_legitimate_merchant_names() {
        let p=pages(&["Account activity\nDate Description Debit Credit Balance\n2026-09-01 Fictional fee 2.05 0.00 97.95\n2026-09-02 Fictional deposit 0.00 100.01 197.96\n2026-09-03 Interest paid 1.01 0.00 196.95\n2026-09-04 NEW BALANCE STORE 10.00 0.00 186.95\n2026-09-05 Ambiguous two active columns 20.00 10.00 176.95"]);
        let a = analyze(&p, Config::default(), vec![]);
        assert_eq!(a.selected.len(), 4);
        assert_eq!(a.summary.outflows, "13.06");
        assert_eq!(a.summary.inflows, "100.01");
        assert_eq!(a.summary.net, "86.95");
        assert!(!a.candidates[4].clear);
        let (e, raw) = capture(&p, Config::default(), a.selected).unwrap();
        assert_eq!(raw[0].reported_debit.as_deref(), Some("2.05"));
        assert_eq!(raw[1].reported_credit.as_deref(), Some("100.01"));
        assert_eq!(replay(&e).unwrap(), raw);
        let p = pages(&[
            "2026-09-01 Merchant $10.01-\n2026-09-02 Merchant 5.02DR\n2026-09-03 Merchant 8.03CR",
        ]);
        let a = analyze(&p, Config::default(), vec![]);
        assert_eq!(a.summary.outflows, "15.03");
        assert_eq!(a.summary.inflows, "8.03");
    }
    #[test]
    fn source_limits_inert_text_and_invalid_or_duplicate_selection() {
        let p = pages(&["2026-09-01 <img onerror=alert(1)> 5.01"]);
        let a = analyze(&p, Config::default(), vec![]);
        assert_eq!(a.selected.len(), 1);
        assert!(a.candidates[0].text.contains("<img"));
        assert!(inspect(
            "c".into(),
            "t".into(),
            &p,
            Config::default(),
            vec![
                Choice {
                    locator: a.selected[0].clone(),
                    checked: true
                },
                Choice {
                    locator: a.selected[0].clone(),
                    checked: false
                }
            ]
        )
        .is_err());
        assert!(capture(
            &p,
            Config::default(),
            vec![a.selected[0].clone(), a.selected[0].clone()]
        )
        .is_err());
        let (mut e, _) = capture(&p, Config::default(), a.selected).unwrap();
        e.version = 9;
        assert!(replay(&e).is_err());
        let c = Config {
            period_start: Some("2026-09-30".into()),
            period_end: Some("2026-09-01".into()),
            ..Config::default()
        };
        assert!(inspect("c".into(), "t".into(), &p, c, vec![]).is_err());
    }
}
