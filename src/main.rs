//! HTTP server that fetches an ical feed and normalizes + filters it.

use crate::env::EnvConfiguration;
use crate::error::Result;
use actix_web::{
    middleware::Logger,
    web::{self, Query},
    App, HttpResponse, HttpServer,
};
use chrono::{DateTime, Utc};
use chrono_tz::{Tz, UTC};
use serde::{Deserialize, Serialize};

pub mod env;
pub mod error;
pub mod upstream;

#[derive(Serialize)]
pub struct Event {
    uid: String,
    summary: String,
    stamp: DateTime<Utc>,
    created: Option<DateTime<Utc>>,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
}

fn instant_to_icalstr(t: &DateTime<Utc>) -> String {
    t.format("%Y%m%dT%H%M%SZ").to_string()
}

impl std::convert::From<Event> for ics::Event<'_> {
    fn from(e: Event) -> Self {
        let mut res = ics::Event::new(e.uid, instant_to_icalstr(&e.stamp));

        use ics::properties::*;
        res.push(Summary::new(e.summary));
        if let Some(start) = e.start.as_ref() {
            res.push(DtStart::new(instant_to_icalstr(&start)));
        }
        if let Some(end) = e.start.as_ref() {
            res.push(DtEnd::new(instant_to_icalstr(&end)));
        }
        if let Some(created) = e.created.as_ref() {
            res.push(Created::new(instant_to_icalstr(created)));
        }

        res
    }
}

async fn compute_events<'a>(
    url: &str,
    selector: &'a str,
) -> Result<impl Iterator<Item = Result<impl Iterator<Item = Result<Event>> + 'a>>> {
    let calendars = upstream::get_calendars(url).await?;

    use ical::parser::ical::component::IcalCalendar;
    Ok(calendars.map(move |c: Result<IcalCalendar>| {
        let c: IcalCalendar = c?;
        Ok(c.events
            .into_iter()
            .map(move |e| -> Result<Option<Event>> {
                let mut uid = None;
                let mut summary = None;
                let mut stamp = None;
                let mut start = None;
                let mut end = None;
                let mut created = None;
                for p in e.properties.into_iter() {
                    let mut tz = UTC;
                    if let Some(params) = p.params {
                        for (name, values) in params {
                            if name == "TZID" && values.len() == 1 {
                                use std::str::FromStr;
                                if let Ok(current_tz) = Tz::from_str(&values[0]) {
                                    tz = current_tz;
                                }
                            }
                        }
                    }

                    let datetime_for_str = |s: String| -> Result<DateTime<Utc>> {
                        use chrono::offset::TimeZone;
                        let dt = tz.datetime_from_str(&s, "%Y%m%dT%H%M%S");
                        if let Ok(dt) = dt {
                            Ok(dt.with_timezone(&Utc))
                        } else {
                            Ok(Utc.datetime_from_str(&s, "%Y%m%dT%H%M%SZ")?)
                        }
                    };

                    match p.name.as_str() {
                        "UID" => uid = p.value,
                        "SUMMARY" => summary = p.value,
                        "DTSTAMP" => stamp = p.value.map(datetime_for_str),
                        "DTSTART" => start = p.value.map(datetime_for_str),
                        "DTEND" => end = p.value.map(datetime_for_str),
                        "CREATED" => created = p.value.map(datetime_for_str),
                        _ => (),
                    }
                }

                if let (Some(uid), Some(summary), Some(stamp)) = (uid, summary, stamp) {
                    if selector == summary {
                        Ok(Some(Event {
                            uid,
                            summary,
                            stamp: stamp?,
                            start: start.transpose()?,
                            end: end.transpose()?,
                            created: created.transpose()?,
                        }))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            })
            .filter_map(|x| -> Option<Result<Event>> { x.transpose() }))
    }))
}

async fn collect_events(url: &str, selector: &str) -> Result<Vec<Event>> {
    let iter = compute_events(url, selector).await?;

    let mut res = Vec::new();
    for calendar in iter {
        for e in calendar? {
            res.push(e?);
        }
    }

    Ok(res)
}

#[derive(Deserialize)]
struct FilterParams {
    url: String,
    filter: String,
}

async fn get_json(query: Query<FilterParams>) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(collect_events(&query.url, &query.filter).await?))
}

async fn get_ical(query: Query<FilterParams>) -> Result<HttpResponse> {
    let events = collect_events(&query.url, &query.filter).await?;

    use ics::{properties::*, *};

    let mut calendar = ICalendar::new("2.0", "ical-filter");
    calendar.add_timezone(TimeZone::new(
        "UTC",
        ZoneTime::standard("19700329T020000", "+0000", "+0000"),
    ));
    calendar.push(CalScale::new("GREGORIAN"));
    calendar.push(Method::new("PUBLISH"));

    for e in events {
        calendar.add_event(e.into());
    }

    Ok(HttpResponse::Ok()
        .content_type("text/calendar")
        .body(calendar.to_string()))
}

#[derive(Clone)]
pub struct Conf(std::sync::Arc<EnvConfiguration>);

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let configuration = env::get_conf().unwrap();
    let socketaddr = configuration.socketaddr;

    HttpServer::new(move || {
        let configuration = configuration.clone();

        App::new()
            .wrap(Logger::default())
            .data(configuration)
            .service(web::resource("/v1/json").to(get_json))
            .service(web::resource("/v1/ical").to(get_ical))
    })
    .bind(socketaddr)?
    .run()
    .await
}
