//! Optional state for the existing savings schedule, not a second forecast engine.
use super::*;
use std::collections::HashSet;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub label: String,
    pub available: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Fund,
    Drawdown,
    Reallocate,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Event {
    pub date: String,
    pub kind: Kind,
    pub bucket: Option<usize>,
    pub destination: Option<usize>,
    pub source_id: Option<String>,
    pub amount: Option<String>,
    pub note: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Plan {
    pub sources: Vec<Source>,
    pub events: Vec<Event>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EventResult {
    pub index: usize,
    pub date: String,
    pub requested: Option<String>,
    pub applied: Option<String>,
    pub shortfall: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Point {
    pub date: String,
    pub emergency: Option<String>,
    pub goals: Vec<Option<String>>,
    pub liquid: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Result {
    pub events: Vec<EventResult>,
    pub points: Vec<Point>,
    pub remaining_sources: Vec<Option<String>>,
    pub warnings: Vec<String>,
}
pub(super) struct State {
    plan: Plan,
    pub emergency: Option<i64>,
    pub goals: Vec<Option<i64>>,
    sources: Vec<Option<i64>>,
    obligations: Option<i64>,
    cursor: usize,
    pub result: Result,
}
fn named(value: &mut String, name: &str, limit: usize) -> std::result::Result<(), String> {
    *value = value.trim().to_owned();
    if value.is_empty() || value.chars().count() > limit || value.chars().any(char::is_control) {
        return Err(format!(
            "{name} needs 1–{limit} characters without control characters."
        ));
    }
    Ok(())
}
pub fn normalize(
    plan: &mut Plan,
    data_goals: usize,
    start: i32,
) -> std::result::Result<(), String> {
    if plan.sources.len() > 20 || plan.events.len() > 100 {
        return Err("Use at most 20 one-time funding sources and 100 savings events.".into());
    }
    let mut ids = HashSet::new();
    let mut labels = HashSet::new();
    for source in &mut plan.sources {
        if source.id.is_empty()
            || source.id.len() > 64
            || !source
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || !ids.insert(source.id.clone())
        {
            return Err("Funding sources need unique valid identifiers.".into());
        }
        named(&mut source.label, "Funding source", 120)?;
        if !labels.insert(source.label.to_lowercase()) {
            return Err(
                "Use a distinct name for each separate funding source; do not count it twice."
                    .into(),
            );
        }
        optional_money(&mut source.available)?;
    }
    for event in &mut plan.events {
        let day = parse_date(&event.date)?;
        if day < NaiveDate::from_ymd_opt(start / 12, (start % 12 + 1) as u32, 1).expect("start")
            || day > end_date(start + HORIZON - 1)
        {
            return Err("Savings events must be within the 600-month projection.".into());
        }
        if event.bucket.is_some_and(|i| i >= data_goals)
            || event.destination.is_some_and(|i| i >= data_goals)
        {
            return Err("Choose an existing savings bucket.".into());
        }
        match event.kind {
            Kind::Fund => {
                if !event.source_id.as_ref().is_some_and(|id| ids.contains(id))
                    || event.destination.is_some()
                {
                    return Err("Funding needs one explicit source and a receiving bucket.".into());
                }
            }
            Kind::Drawdown => {
                if event.source_id.is_some() || event.destination.is_some() {
                    return Err("Drawdowns use only their selected existing bucket.".into());
                }
            }
            Kind::Reallocate => {
                if event.source_id.is_some() || event.bucket == event.destination {
                    return Err(
                        "Reallocation needs two distinct existing buckets and no external source."
                            .into(),
                    );
                }
            }
        }
        optional_money(&mut event.amount)?;
        named(&mut event.note, "Savings event purpose", 300)?;
    }
    // Stable order on equal dates preserves the counselor's explicit funding sequence.
    plan.events.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(())
}
impl State {
    pub fn new(
        data: &PlanData,
        emergency: Option<i64>,
    ) -> std::result::Result<Option<Self>, String> {
        let Some(plan) = data.funding.clone() else {
            return Ok(None);
        };
        let parse = |v: &Option<String>| v.as_deref().map(parse_money).transpose();
        let goals = data
            .goals
            .iter()
            .map(|g| parse(&g.reserved_amount))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let sources = plan
            .sources
            .iter()
            .map(|s| parse(&s.available))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut state=Self {emergency,goals,sources,obligations:parse(&data.obligations_reserved)?,cursor:0,plan,result:Result{events:vec![],points:vec![],remaining_sources:vec![],warnings:vec!["Hypothetical reserve movements only. External source balances must be separate from opening liquid savings, recurring income and each other. No interest, bank transfer, expense, debt payment or live balance is recorded. Drawdowns and reallocation cannot use more than the available selected bucket; unfunded amounts remain explicit shortfalls. Events precede month-end contributions, including on the same day. Reconcile any spending separately with the budget/calendar; do not enter it twice.".into()]}};
        state.point(&format!("{}-01", data.start_month));
        Ok(Some(state))
    }
    fn bucket(&self, index: Option<usize>) -> Option<i64> {
        index.map_or(self.emergency, |i| self.goals[i])
    }
    fn set_bucket(&mut self, index: Option<usize>, value: Option<i64>) {
        if let Some(i) = index {
            self.goals[i] = value;
        } else {
            self.emergency = value;
        }
    }
    pub fn apply(&mut self, through: NaiveDate) -> std::result::Result<(), String> {
        while self.cursor < self.plan.events.len()
            && self.plan.events[self.cursor].date <= through.to_string()
        {
            let event = self.plan.events[self.cursor].clone();
            let amount = event.amount.as_deref().map(parse_money).transpose()?;
            let source_index = event
                .source_id
                .as_ref()
                .and_then(|id| self.plan.sources.iter().position(|s| &s.id == id));
            let available = if event.kind == Kind::Fund {
                source_index.and_then(|i| self.sources[i])
            } else {
                self.bucket(event.bucket)
            };
            let applied = available.zip(amount).map(|(a, r)| a.min(r));
            let shortfall = amount.zip(applied).map(|(r, a)| r - a);
            match event.kind {
                Kind::Fund => {
                    let i = source_index.expect("validated source");
                    self.sources[i] = available.zip(applied).map(|(v, a)| v - a);
                    self.set_bucket(
                        event.bucket,
                        self.bucket(event.bucket).zip(applied).map(|(v, a)| v + a),
                    );
                }
                Kind::Drawdown => {
                    self.set_bucket(event.bucket, available.zip(applied).map(|(v, a)| v - a))
                }
                Kind::Reallocate => {
                    self.set_bucket(event.bucket, available.zip(applied).map(|(v, a)| v - a));
                    self.set_bucket(
                        event.destination,
                        self.bucket(event.destination)
                            .zip(applied)
                            .map(|(v, a)| v + a),
                    );
                }
            }
            if shortfall.is_some_and(|v| v > 0) {
                self.result.warnings.push(format!("{}: savings event {} has an unfunded shortfall of {}. No missing funds were created.",event.date,self.cursor+1,money(shortfall.unwrap_or(0))));
            }
            if applied.is_none() {
                self.result.warnings.push(format!("{}: event {} has unknown funding; affected balances and milestones are unavailable.",event.date,self.cursor+1));
            }
            self.result.events.push(EventResult {
                index: self.cursor,
                date: event.date.clone(),
                requested: amount.map(money),
                applied: applied.map(money),
                shortfall: shortfall.map(money),
            });
            self.point(&event.date);
            self.cursor += 1;
        }
        Ok(())
    }
    pub fn point(&mut self, date: &str) {
        let liquid = self.goals.iter().fold(
            self.emergency.zip(self.obligations).map(|(a, b)| a + b),
            |total, v| total.zip(*v).map(|(a, b)| a + b),
        );
        self.result.points.push(Point {
            date: date.into(),
            emergency: self.emergency.map(money),
            goals: self.goals.iter().map(|v| v.map(money)).collect(),
            liquid: liquid.map(money),
        });
    }
    pub fn future(&self) -> bool {
        self.cursor < self.plan.events.len()
    }
    pub fn finish(mut self) -> Result {
        self.result.remaining_sources = self.sources.iter().map(|v| v.map(money)).collect();
        self.result
    }
}
pub fn forecast_from_points(
    original: Forecast,
    target: Option<i64>,
    bucket: Option<usize>,
    points: &[Point],
    start: i32,
) -> Forecast {
    let value = |p: &Point| {
        bucket
            .map_or(p.emergency.as_deref(), |i| p.goals[i].as_deref())
            .map(crate::budget::parse_total)
            .transpose()
            .ok()
            .flatten()
    };
    if points.is_empty() || original.status == ForecastStatus::Unknown {
        return Forecast {
            status: ForecastStatus::Unknown,
            months_needed: None,
            completion_date: None,
            ..original
        };
    }
    let Some(target) = target else {
        return Forecast {
            status: ForecastStatus::Unknown,
            months_needed: None,
            completion_date: None,
            ..original
        };
    };
    if points.iter().any(|p| value(p).is_none()) {
        return Forecast {
            status: ForecastStatus::Unknown,
            months_needed: None,
            completion_date: None,
            ..original
        };
    }
    let last_below = points
        .iter()
        .rposition(|p| value(p).is_some_and(|v| v < target));
    if last_below.is_none() && original.status == ForecastStatus::Funded {
        return original;
    }
    let reached = points
        .iter()
        .skip(last_below.map_or(0, |i| i + 1))
        .find(|p| value(p).is_some_and(|v| v >= target));
    if let Some(point) = reached {
        let date = NaiveDate::parse_from_str(&point.date, "%Y-%m-%d").expect("generated date");
        let number = date.year() * 12 + date.month() as i32 - 1;
        Forecast {
            status: ForecastStatus::Projected,
            months_needed: Some((number - start + 1) as u32),
            completion_date: Some(point.date.clone()),
            ..original
        }
    } else {
        Forecast {
            status: ForecastStatus::HorizonReached,
            months_needed: None,
            completion_date: None,
            ..original
        }
    }
}
