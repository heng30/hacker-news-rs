use crate::{components::icons, models::Episode};
use chrono::Datelike;
use leptos::prelude::*;

#[component]
pub fn CalendarModal(
    is_open: ReadSignal<bool>,
    episodes: ReadSignal<Vec<Episode>>,
    selected_date: ReadSignal<Option<String>>,
    on_select_date: Callback<String>,
    on_close: Callback<()>,
) -> impl IntoView {
    let now = chrono::Local::now();
    let (view_year, set_view_year) = signal(now.year());
    let (view_month, set_view_month) = signal(now.month() as i32);

    let day_headers = move || {
        let headers = vec!["日", "一", "二", "三", "四", "五", "六"];
        headers
            .into_iter()
            .map(|d| {
                view! { <div class="calendar-day-header">{d}</div> }
            })
            .collect::<Vec<_>>()
    };

    let calendar_days = move || {
        let year = view_year.get();
        let month = view_month.get() as u32;

        let first_day = chrono::NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let next_month_first = if month == 12 {
            chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
        } else {
            chrono::NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
        };
        let days_in_month = (next_month_first - first_day).num_days() as u32;

        let start_weekday = first_day.weekday().num_days_from_sunday();
        let prev_month_last = first_day.pred_opt().unwrap_or(first_day);

        let episode_dates: std::collections::HashSet<String> =
            episodes.get().iter().map(|e| e.date.clone()).collect();
        let today = chrono::Local::now().date_naive();

        let mut cells: Vec<AnyView> = Vec::new();

        // Previous month days
        for i in (0..start_weekday).rev() {
            let day = prev_month_last.day() as u32 - i;
            cells.push(
                view! {
                    <div class="calendar-day other-month empty">{day}</div>
                }
                .into_any(),
            );
        }

        // Current month days
        for day in 1..=days_in_month {
            let date_str = format!("{}-{:02}-{:02}", year, month, day);
            let is_today = year == today.year() && month == today.month() && day == today.day();
            let has_data = episode_dates.contains(&date_str);
            let is_selected = selected_date.get().as_ref() == Some(&date_str);

            let mut classes = vec!["calendar-day"];
            if is_today {
                classes.push("today");
            }
            if has_data {
                classes.push("has-data");
            }
            if is_selected {
                classes.push("selected");
            }
            let class_str = classes.join(" ");

            let date_clone = date_str.clone();
            cells.push(
                view! {
                    <div
                        class=class_str
                        on:click=move |_| on_select_date.run(date_clone.clone())
                    >
                        {day}
                    </div>
                }
                .into_any(),
            );
        }

        // Next month days
        let last_day =
            chrono::NaiveDate::from_ymd_opt(year, month, days_in_month).unwrap_or(first_day);
        let end_weekday = last_day.weekday().num_days_from_sunday();
        if end_weekday < 6 {
            for i in 1..(7 - end_weekday) {
                cells.push(
                    view! {
                        <div class="calendar-day other-month empty">{i}</div>
                    }
                    .into_any(),
                );
            }
        }

        cells
    };

    let month_title = move || {
        let year = view_year.get();
        let month = view_month.get() as u32;
        let date = chrono::NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        date.format("%B %Y").to_string()
    };

    let prev_month = move || {
        let m = view_month.get_untracked();
        let y = view_year.get_untracked();
        if m <= 1 {
            set_view_month.set(12);
            set_view_year.set(y - 1);
        } else {
            set_view_month.set(m - 1);
        }
    };

    let next_month = move || {
        let m = view_month.get_untracked();
        let y = view_year.get_untracked();
        if m >= 12 {
            set_view_month.set(1);
            set_view_year.set(y + 1);
        } else {
            set_view_month.set(m + 1);
        }
    };

    let overlay_class = move || {
        if is_open.get() {
            "modal-overlay active"
        } else {
            "modal-overlay"
        }
    };

    view! {
        <div class=overlay_class on:click=move |e| {
            if e.target() == e.current_target() {
                on_close.run(());
            }
        }>
            <div class="modal calendar-modal">
                <div class="modal-header">
                    <h2>
                        <span class="icon" inner_html=icons::CALENDAR_WITH_STYLE></span>
                        "日历"
                    </h2>
                    <button class="modal-close" on:click=move |_| on_close.run(())>"×"</button>
                </div>
                <div class="calendar-card-modal">
                    <div class="calendar-header">
                        <button class="calendar-nav" on:click=move |_| prev_month()>
                            <span class="icon" inner_html=icons::CHEVRON_LEFT></span>
                        </button>
                        <h3>{month_title}</h3>
                        <button class="calendar-nav" on:click=move |_| next_month()>
                            <span class="icon" inner_html=icons::CHEVRON_RIGHT></span>
                        </button>
                    </div>
                    <div class="calendar-grid">
                        {day_headers()}
                        {calendar_days}
                    </div>
                </div>
            </div>
        </div>
    }
}
