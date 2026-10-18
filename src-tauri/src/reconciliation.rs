//! Exact running-balance comparisons. Reported source figures are references,
//! never inferred balances or automatically approved financial facts.
use crate::budget::money;
use crate::imports::{iso_date, signed_money, DocumentInput, Record, Status};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Order {
    OldestFirst,
    NewestFirst,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Review {
    pub id: String,
    pub document_id: String,
    pub order: Option<Order>,
    pub reason: String,
    pub reviewed_at: String,
    pub counselor_name: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewInput {
    pub expected_revision: i64,
    pub document_id: String,
    pub order: Option<Order>,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq, Deserialize)]
pub enum State {
    Matched,
    Difference,
    Unknown,
    Pending,
    Excluded,
    NotConfigured,
    OrderConflict,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub state: State,
    pub expected: Option<String>,
    pub difference: Option<String>,
    pub note: String,
}
impl Comparison {
    fn unavailable(state: State, note: &str) -> Self {
        Self {
            state,
            expected: None,
            difference: None,
            note: note.into(),
        }
    }
    fn compare(expected: i64, reported: Option<i64>) -> Self {
        match reported {
            Some(reported) => Self {state:if expected==reported {State::Matched} else {State::Difference},expected:Some(money(expected)),difference:Some(money(expected-reported)),note:"Expected minus reported, in exact USD cents. Arithmetic agreement does not prove statement completeness or bank verification.".into()},
            None => Self {state:State::Unknown,expected:Some(money(expected)),difference:None,note:"Reported running balance is missing or invalid. No zero difference is assumed.".into()},
        }
    }
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
pub enum Sequence {
    Valid,
    Unknown,
    Conflict,
    NotConfigured,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub record_id: String,
    pub reported: Option<String>,
    pub source: Comparison,
    pub approved: Comparison,
}
#[derive(Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub order: Option<Order>,
    pub source_sequence: Sequence,
    pub approved_sequence: Sequence,
    pub rows: Vec<Row>,
    pub source_matched: usize,
    pub source_differences: usize,
    pub approved_matched: usize,
    pub approved_differences: usize,
    pub reported_closing: Comparison,
    pub warnings: Vec<String>,
}
impl Report {
    pub fn conflicts(&self) -> bool {
        self.source_differences > 0
            || self.approved_differences > 0
            || self.source_sequence == Sequence::Conflict
            || self.approved_sequence == Sequence::Conflict
            || self.reported_closing.state == State::Difference
    }
}
fn sequence(rows: &[&Record], configured: bool, approved: bool) -> Sequence {
    if !configured {
        return Sequence::NotConfigured;
    }
    let dates: Option<Vec<_>> = rows
        .iter()
        .filter(|r| !approved || r.status != Status::Rejected)
        .map(|r| {
            let fields = if approved && r.status == Status::Approved {
                &r.fields
            } else {
                &r.raw.extracted
            };
            fields.date.as_deref().and_then(|v| iso_date(v).ok())
        })
        .collect();
    match dates {
        None => Sequence::Unknown,
        Some(dates) if dates.windows(2).any(|p| p[0] > p[1]) => Sequence::Conflict,
        Some(_) => Sequence::Valid,
    }
}
fn order_error(sequence: &Sequence) -> Option<Comparison> {
    match sequence {
        Sequence::NotConfigured => Some(Comparison::unavailable(State::NotConfigured,"Declare source order and confirm balances are after each transaction before checking rows.")),
        Sequence::Unknown => Some(Comparison::unavailable(State::Unknown,"A source date is unknown; chronological source order cannot be validated.")),
        Sequence::Conflict => Some(Comparison::unavailable(State::OrderConflict,"Dates conflict with the declared source order. Rows have not been silently sorted.")),
        Sequence::Valid => None,
    }
}
fn amount(v: &Option<String>) -> Result<Option<i64>, String> {
    v.as_deref().map(signed_money).transpose()
}
fn add(a: i64, b: i64) -> Result<i64, String> {
    a.checked_add(b)
        .ok_or("Statement balance exceeds the supported range.".into())
}
pub fn check(
    input: &DocumentInput,
    rows: &[&Record],
    order: Option<Order>,
) -> Result<Report, String> {
    let mut chronological = rows.to_vec();
    chronological.sort_by_key(|r| r.raw.record_number);
    if order == Some(Order::NewestFirst) {
        chronological.reverse();
    }
    let configured =
        order.is_some() && input.pdf_selection.is_none() && input.mapping.balance.is_some();
    let source_sequence = sequence(&chronological, configured, false);
    let approved_sequence = sequence(&chronological, configured, true);
    let opening = amount(&input.opening_balance)?;
    let closing = amount(&input.closing_balance)?;
    let mut previous = opening;
    let mut cumulative = opening;
    let mut checks = Vec::new();
    for r in &chronological {
        let reported = amount(&r.raw.balance)?;
        let source = if let Some(error) = order_error(&source_sequence) {
            error
        } else {
            match previous.zip(amount(&r.raw.extracted.amount)?) {
                Some((before,movement)) => Comparison::compare(add(before,movement)?,reported),
                None => Comparison::unavailable(State::Unknown,"Previous reported balance or original amount is unknown; this local step cannot be checked."),
            }
        };
        previous = reported; // Re-anchor only to a reported figure, never a predicted one.
        let approved=match r.status {
            Status::Rejected => Comparison::unavailable(State::Excluded,"Rejected transaction contributes no approved movement; its reported balance is not used as an approved anchor."),
            Status::Pending|Status::NeedsClarification => {
                cumulative=None;
                Comparison::unavailable(State::Pending,"Unapproved movement is unknown. Later approved balances remain incomplete until this source row is decided.")
            },
            Status::Approved => {
                cumulative=match cumulative.zip(amount(&r.fields.amount)?) {Some((before,movement))=>Some(add(before,movement)?),None=>None};
                if let Some(error)=order_error(&approved_sequence) {error} else {match cumulative {
                    Some(expected)=>Comparison::compare(expected,reported),
                    None=>Comparison::unavailable(State::Unknown,"Opening balance or an earlier decision is incomplete. Reported source balances are not substituted for missing approved data."),
                }}
            },
        };
        checks.push(Row {
            record_id: r.id.clone(),
            reported: r.raw.balance.clone(),
            source,
            approved,
        });
    }
    let reported_closing = if let Some(error) = order_error(&source_sequence) {
        error
    } else {
        match chronological
            .last()
            .and_then(|r| r.raw.balance.as_ref())
            .map(|v| signed_money(v))
            .transpose()?
            .zip(closing)
        {
            Some((last, close)) => Comparison::compare(last, Some(close)),
            None => Comparison::unavailable(
                State::Unknown,
                "Last reported balance or closing statement balance is unknown.",
            ),
        }
    };
    let mut warnings=vec!["Balances must be signed checking/savings balances after each transaction, not available balances, credit limits, or before-transaction balances. These checks do not approve balances, verify an institution, or prove every transaction was imported.".into()];
    if !configured {
        warnings.push("Running-balance checks require a mapped CSV balance column and an explicit source order. The current text-PDF parser does not extract running-balance columns.".into());
    }
    if opening.is_none() {
        warnings.push("Opening balance is unknown: approved cumulative comparisons are unavailable. Independent local source steps may still be checked from reported anchors.".into());
    }
    if chronological.windows(2).any(|p| {
        p[0].raw.extracted.date.is_some() && p[0].raw.extracted.date == p[1].raw.extracted.date
    }) {
        warnings.push("Same-day transactions keep the declared file sequence. Dates alone cannot establish their posting order.".into());
    }
    let count = |approved: bool, state: State| {
        checks
            .iter()
            .filter(|r| {
                if approved {
                    r.approved.state == state
                } else {
                    r.source.state == state
                }
            })
            .count()
    };
    Ok(Report {
        order,
        source_sequence,
        approved_sequence,
        source_matched: count(false, State::Matched),
        source_differences: count(false, State::Difference),
        approved_matched: count(true, State::Matched),
        approved_differences: count(true, State::Difference),
        reported_closing,
        rows: checks,
        warnings,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::imports::{extract, read_csv, tests::mapping};
    const SOURCE:&str="Date,Description,Amount,Category,Type,Balance\n2026-09-01,Fictional wages,500.01,Wages,income,1500.01\n2026-09-02,Fictional rent,-300.02,Housing,expense,1199.99\n2026-09-03,Fictional fee,-0.01,Fees,fee,1199.98\n";
    pub(crate) fn fixture() -> (DocumentInput, Vec<Record>) {
        let mut mapping = mapping();
        mapping.balance = Some(5);
        let input = DocumentInput {
            balance_order: None,
            pdf_selection: None,
            pdf: None,
            account_id: None,
            account_label: "Fictional checking".into(),
            account_kind: "checking".into(),
            period_start: Some("2026-09-01".into()),
            period_end: Some("2026-09-30".into()),
            opening_balance: Some("1000.00".into()),
            closing_balance: Some("1199.98".into()),
            retain_source: false,
            mapping,
        };
        let rows = extract(&read_csv(SOURCE.as_bytes()).unwrap(), &input.mapping)
            .unwrap()
            .into_iter()
            .enumerate()
            .map(|(i, raw)| Record {
                id: format!("row-{i}"),
                document_id: "doc".into(),
                account_id: "account".into(),
                fields: raw.extracted.clone(),
                raw,
                status: Status::Approved,
                linked_id: None,
                reason: "Verified source".into(),
                warnings: vec![],
                duplicate_ids: vec![],
                bulk_eligible: false,
            })
            .collect();
        (input, rows)
    }
    fn report(input: &DocumentInput, rows: &[Record], order: Option<Order>) -> Report {
        check(input, &rows.iter().collect::<Vec<_>>(), order).unwrap()
    }
    #[test]
    fn independent_exact_source_and_approved_steps_in_both_declared_orders() {
        let (input, mut rows) = fixture();
        let r = report(&input, &rows, Some(Order::OldestFirst));
        assert_eq!((r.source_matched, r.approved_matched), (3, 3));
        assert_eq!(r.rows[0].source.expected.as_deref(), Some("1500.01"));
        assert_eq!(r.rows[1].source.expected.as_deref(), Some("1199.99"));
        assert_eq!(r.rows[2].approved.expected.as_deref(), Some("1199.98"));
        assert_eq!(r.reported_closing.difference.as_deref(), Some("0.00"));
        assert!(!r.conflicts());
        rows.reverse();
        for (i, row) in rows.iter_mut().enumerate() {
            row.raw.record_number = i as u32 + 2;
            row.raw.line = i as u64 + 2;
        }
        let reverse = report(&input, &rows, Some(Order::NewestFirst));
        assert_eq!((reverse.source_matched, reverse.approved_matched), (3, 3));
        assert_eq!(reverse.rows[0].record_id, "row-0");
        let wrong = report(&input, &rows, Some(Order::OldestFirst));
        assert_eq!(wrong.source_sequence, Sequence::Conflict);
        assert!(wrong
            .rows
            .iter()
            .all(|r| r.source.expected.is_none() && r.approved.expected.is_none()));
        assert!(wrong.conflicts());
        let unknown = report(&input, &rows, None);
        assert_eq!(unknown.source_sequence, Sequence::NotConfigured);
        assert!(unknown.rows.iter().all(|r| r.source.difference.is_none()));
    }
    #[test]
    fn missing_figures_dates_and_pending_rows_never_create_approved_anchors() {
        let (mut input, mut rows) = fixture();
        input.opening_balance = None;
        let r = report(&input, &rows, Some(Order::OldestFirst));
        assert_eq!(r.source_matched, 2);
        assert_eq!(r.approved_matched, 0);
        assert!(r.rows.iter().all(|r| r.approved.expected.is_none()));
        input.opening_balance = Some("1000.00".into());
        rows[0].raw.balance = None;
        let r = report(&input, &rows, Some(Order::OldestFirst));
        assert_eq!(r.source_matched, 1);
        assert_eq!(r.rows[0].source.expected.as_deref(), Some("1500.01"));
        assert!(r.rows[0].source.difference.is_none());
        assert!(r.rows[1].source.expected.is_none());
        assert_eq!(r.approved_matched, 2);
        rows[0].status = Status::Pending;
        let r = report(&input, &rows, Some(Order::OldestFirst));
        assert_eq!(r.rows[0].approved.state, State::Pending);
        assert!(r.rows.iter().all(|r| r.approved.expected.is_none()));
        rows[0].status = Status::Approved;
        rows[0].raw.extracted.date = None;
        let r = report(&input, &rows, Some(Order::OldestFirst));
        assert_eq!(r.source_sequence, Sequence::Unknown);
        assert_eq!(r.approved_sequence, Sequence::Valid);
        assert_eq!(r.approved_matched, 2);
        rows[0].status = Status::Rejected;
        let rejected_date = report(&input, &rows, Some(Order::OldestFirst));
        assert_eq!(rejected_date.source_sequence, Sequence::Unknown);
        assert_eq!(rejected_date.approved_sequence, Sequence::Valid);
        assert_eq!(rejected_date.rows[0].approved.state, State::Excluded);
        assert_eq!(
            rejected_date.rows[2].approved.expected.as_deref(),
            Some("699.97")
        );
        rows[0].status = Status::Approved;
        rows[0].raw.extracted.date = Some("2026-09-01".into());
        rows[0].raw.extracted.amount = None;
        let r = report(&input, &rows, Some(Order::OldestFirst));
        assert!(r.rows[0].source.expected.is_none());
        assert_eq!(r.rows[0].approved.expected.as_deref(), Some("1500.01"));
    }
    #[test]
    fn intermediate_discrepancies_corrections_rejections_and_closing_checks_are_distinct() {
        let (input, mut rows) = fixture();
        rows[1].raw.balance = Some("1200.00".into());
        let r = report(&input, &rows, Some(Order::OldestFirst));
        assert_eq!(r.source_differences, 2);
        assert_eq!(r.rows[1].source.difference.as_deref(), Some("-0.01"));
        assert_eq!(r.rows[2].source.difference.as_deref(), Some("0.01"));
        assert_eq!(r.reported_closing.state, State::Matched);
        rows[1].raw.balance = Some("1199.99".into());
        rows[1].fields.amount = Some("-300.01".into());
        let r = report(&input, &rows, Some(Order::OldestFirst));
        assert_eq!(r.source_matched, 3);
        assert_eq!(r.approved_differences, 2);
        assert_eq!(r.rows[2].approved.expected.as_deref(), Some("1199.99"));
        assert_eq!(r.rows[2].approved.difference.as_deref(), Some("0.01"));
        rows[1].fields.date = Some("2026-08-31".into());
        let dates = report(&input, &rows, Some(Order::OldestFirst));
        assert_eq!(dates.source_matched, 3);
        assert_eq!(dates.approved_sequence, Sequence::Conflict);
        assert!(dates.rows.iter().all(|r| r.approved.expected.is_none()));
        rows[1].fields.date = Some("2026-09-02".into());
        rows[1].status = Status::Rejected;
        let r = report(&input, &rows, Some(Order::OldestFirst));
        assert_eq!(r.rows[1].approved.state, State::Excluded);
        assert!(r.rows[1].approved.expected.is_none());
        assert_eq!(r.rows[2].approved.expected.as_deref(), Some("1500.00"));
        assert_eq!(r.rows[2].approved.difference.as_deref(), Some("300.02"));
        let mut incorrect_close = input.clone();
        incorrect_close.closing_balance = Some("1200.00".into());
        let r = report(&incorrect_close, &rows, Some(Order::OldestFirst));
        assert_eq!(r.reported_closing.difference.as_deref(), Some("-0.02"));
        assert!(r.conflicts());
    }
    #[test]
    fn same_day_order_signed_debits_negative_balances_and_known_zero_are_preserved() {
        let (mut input, _) = fixture();
        input.mapping.signed_is_debit = true;
        input.opening_balance = Some("0.00".into());
        input.closing_balance = Some("0.00".into());
        let text="Date,Description,Amount,Category,Type,Balance\n2026-09-01,Fictional debit,25.11,Food,expense,-25.11\n2026-09-01,Fictional deposit,-25.11,Wages,income,0.00\n";
        let (_, base) = fixture();
        let rows: Vec<_> = extract(&read_csv(text.as_bytes()).unwrap(), &input.mapping)
            .unwrap()
            .into_iter()
            .enumerate()
            .map(|(i, raw)| Record {
                fields: raw.extracted.clone(),
                raw,
                ..clone_record(&base[i])
            })
            .collect();
        let r = report(&input, &rows, Some(Order::OldestFirst));
        assert_eq!(r.rows[0].source.expected.as_deref(), Some("-25.11"));
        assert_eq!(r.rows[1].source.expected.as_deref(), Some("0.00"));
        assert_eq!((r.source_matched, r.approved_matched), (2, 2));
        assert!(r.warnings.iter().any(|w| w.contains("Same-day")));
        let mut value = serde_json::to_value(&input).unwrap();
        value.as_object_mut().unwrap().remove("balanceOrder");
        assert!(serde_json::from_value::<DocumentInput>(value)
            .unwrap()
            .balance_order
            .is_none());
        assert!(serde_json::from_str::<Order>("\"automatic\"").is_err());
    }
    fn clone_record(r: &Record) -> Record {
        Record {
            id: r.id.clone(),
            document_id: r.document_id.clone(),
            account_id: r.account_id.clone(),
            fields: r.fields.clone(),
            raw: r.raw.clone(),
            status: r.status.clone(),
            linked_id: r.linked_id.clone(),
            reason: r.reason.clone(),
            warnings: r.warnings.clone(),
            duplicate_ids: r.duplicate_ids.clone(),
            bulk_eligible: r.bulk_eligible,
        }
    }
}
