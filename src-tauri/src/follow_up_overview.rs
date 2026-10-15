//! Bounded, date-only practice reminders derived from saved client care.
use crate::models::WorkspaceInfo;
use serde::{Deserialize, Serialize};

pub const PAGE_SIZE: usize = 25;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub view: String,
    pub kind: String,
    pub include_archived: bool,
    pub search: String,
    pub page: usize,
}

pub fn clean(mut i: Input) -> Result<Input, String> {
    i.search = i.search.trim().into();
    if !matches!(
        i.view.as_str(),
        "open" | "overdue" | "today" | "upcoming" | "closed"
    ) || !matches!(i.kind.as_str(), "all" | "task" | "appointment")
        || i.page >= 4000
        || i.search.chars().count() > 100
        || i.search.chars().any(char::is_control)
    {
        return Err("Choose valid follow-up filters, a search of at most 100 characters and a supported page.".into());
    }
    Ok(i)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub client_id: String,
    pub client_name: String,
    pub client_status: String,
    pub follow_up_id: String,
    pub kind: String,
    pub title: String,
    pub due_date: String,
    pub time: String,
    pub status: String,
    pub care_revision: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub organization_id: String,
    pub counselor_id: String,
    pub review_date: String,
    pub time_zone: String,
    pub input: Input,
    pub overdue: usize,
    pub due_today: usize,
    pub upcoming: usize,
    pub closed: usize,
    pub total: usize,
    pub page_size: usize,
    pub items: Vec<Item>,
}

pub fn build(input: Input, mut items: Vec<Item>, today: &str, w: &WorkspaceInfo) -> Overview {
    let mut overdue = 0;
    let mut due_today = 0;
    let mut upcoming = 0;
    let mut closed = 0;
    for f in &items {
        if f.status != "open" {
            closed += 1;
        } else {
            match f.due_date.as_str().cmp(today) {
                std::cmp::Ordering::Less => overdue += 1,
                std::cmp::Ordering::Equal => due_today += 1,
                std::cmp::Ordering::Greater => upcoming += 1,
            }
        }
    }
    let search = input.search.to_lowercase();
    items.retain(|f| {
        let view = match input.view.as_str() {
            "closed" => f.status != "open",
            "overdue" => f.status == "open" && f.due_date.as_str() < today,
            "today" => f.status == "open" && f.due_date == today,
            "upcoming" => f.status == "open" && f.due_date.as_str() > today,
            _ => f.status == "open",
        };
        view && (input.kind == "all" || input.kind == f.kind)
            && (search.is_empty()
                || f.client_name.to_lowercase().contains(&search)
                || f.title.to_lowercase().contains(&search))
    });
    items.sort_by_cached_key(|f| {
        (
            f.due_date.clone(),
            f.time.clone(),
            f.client_name.to_lowercase(),
            f.client_id.clone(),
            f.follow_up_id.clone(),
        )
    });
    let total = items.len();
    let page = input.page.min(total.saturating_sub(1) / PAGE_SIZE);
    let items = items
        .into_iter()
        .skip(page * PAGE_SIZE)
        .take(PAGE_SIZE)
        .collect();
    Overview {
        organization_id: w.organization_id.clone(),
        counselor_id: w.counselor_id.clone(),
        review_date: today.into(),
        time_zone: "America/Chicago".into(),
        input: Input { page, ..input },
        overdue,
        due_today,
        upcoming,
        closed,
        total,
        page_size: PAGE_SIZE,
        items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> Input {
        Input {
            view: "open".into(),
            kind: "all".into(),
            include_archived: false,
            search: "".into(),
            page: 0,
        }
    }
    fn workspace() -> WorkspaceInfo {
        WorkspaceInfo {
            organization_id: "org".into(),
            counselor_id: "counselor".into(),
            organization_name: "Fictional org".into(),
            counselor_name: "Fictional counselor".into(),
        }
    }
    fn item(id: usize, date: &str, status: &str) -> Item {
        Item {
            client_id: "client".into(),
            client_name: "Same Name".into(),
            client_status: "Planning".into(),
            follow_up_id: format!("{id:03}"),
            kind: "task".into(),
            title: format!("Fictional task {id}"),
            due_date: date.into(),
            time: "".into(),
            status: status.into(),
            care_revision: 1,
        }
    }
    #[test]
    fn exact_counts_filters_and_closed_history_are_separate() {
        let items = vec![
            item(0, "2026-10-14", "open"),
            item(1, "2026-10-15", "open"),
            item(2, "2026-10-16", "open"),
            item(3, "2026-10-01", "completed"),
            item(4, "2026-10-01", "cancelled"),
        ];
        let w = workspace();
        let today = "2026-10-15";
        let r = build(input(), items.clone(), today, &w);
        assert_eq!(
            (r.overdue, r.due_today, r.upcoming, r.closed, r.total),
            (1, 1, 1, 2, 3)
        );
        for (view, expected) in [("overdue", 0), ("today", 1), ("upcoming", 2)] {
            let r = build(
                Input {
                    view: view.into(),
                    ..input()
                },
                items.clone(),
                today,
                &w,
            );
            assert_eq!(r.items[0].follow_up_id, format!("{expected:03}"));
            assert_eq!(r.total, 1);
        }
        let r = build(
            Input {
                view: "closed".into(),
                search: "task 4".into(),
                ..input()
            },
            items,
            today,
            &w,
        );
        assert_eq!(
            (r.total, r.closed, r.items[0].status.as_str()),
            (1, 2, "cancelled")
        );
    }
    #[test]
    fn stable_pagination_does_not_invent_times_or_lose_duplicate_names() {
        let mut items: Vec<_> = (0..31)
            .rev()
            .map(|i| item(i, "2026-10-15", "open"))
            .collect();
        items[0].kind = "appointment".into();
        items[0].time = "09:30".into();
        let r = build(input(), items.clone(), "2026-10-15", &workspace());
        assert_eq!(r.total, 31);
        assert_eq!(r.items.len(), 25);
        assert_eq!(r.items[0].follow_up_id, "000");
        assert_eq!(r.items[0].time, "");
        let next = build(
            Input { page: 1, ..input() },
            items.clone(),
            "2026-10-15",
            &workspace(),
        );
        assert_eq!(next.items.len(), 6);
        assert_eq!(next.items[5].time, "09:30");
        let r = build(
            Input {
                page: 3,
                kind: "appointment".into(),
                search: "same NAME".into(),
                ..input()
            },
            items,
            "2026-10-15",
            &workspace(),
        );
        assert_eq!((r.input.page, r.total), (0, 1));
        assert_eq!(r.items[0].kind, "appointment");
        assert!(clean(Input {
            page: 4000,
            ..input()
        })
        .is_err());
        assert!(clean(Input {
            search: "x".repeat(101),
            ..input()
        })
        .is_err());
        assert!(clean(Input {
            view: "all private notes".into(),
            ..input()
        })
        .is_err());
    }
}
