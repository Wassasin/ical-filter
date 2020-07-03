//! HTTP server that fetches an ical feed and normalizes + filters it.

use crate::env::EnvConfiguration;
use crate::error::Result;
use actix_web::{
    middleware::Logger,
    web::{self, Data, Path, Query},
    App, HttpResponse, HttpServer,
};
use serde::{Deserialize, Serialize};

pub mod env;
pub mod error;
pub mod upstream;

#[derive(Serialize)]
pub struct Event {
    uid: String,
    summary: String,
    stamp: String,
    start: String,
    end: String,
}

impl std::convert::From<Event> for ics::Event<'_> {
    fn from(e: Event) -> Self {
        let mut res = ics::Event::new(e.uid, e.stamp);

        use ics::properties::*;
        res.push(Summary::new(e.summary));
        res.push(DtStart::new(e.start));
        res.push(DtEnd::new(e.end));

        res
    }
}

async fn compute_events<'a>(
    url: &str,
    selector: &'a str,
) -> Result<impl Iterator<Item = Result<impl Iterator<Item = Event> + 'a>>> {
    let calendars = upstream::get_calendars(url).await?;

    use ical::parser::ical::component::IcalCalendar;
    Ok(calendars.map(move |c: Result<IcalCalendar>| {
        let c: IcalCalendar = c?;
        Ok(c.events.into_iter().filter_map(move |e| {
            let mut uid = None;
            let mut summary = None;
            let mut stamp = None;
            let mut start = None;
            let mut end = None;
            for p in e.properties.into_iter() {
                match p.name.as_str() {
                    "UID" => uid = p.value,
                    "SUMMARY" => summary = p.value,
                    "DTSTAMP" => stamp = p.value,
                    "DTSTART" => start = p.value,
                    "DTEND" => end = p.value,
                    _ => (),
                }
            }

            if let (Some(uid), Some(summary), Some(stamp), Some(start), Some(end)) =
                (uid, summary, stamp, start, end)
            {
                if selector == summary {
                    Some(Event {
                        uid,
                        summary,
                        stamp,
                        start,
                        end,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        }))
    }))
}

async fn collect_events(url: &str, selector: &str) -> Result<Vec<Event>> {
    let iter = compute_events(url, selector).await?;

    let mut res = Vec::new();
    for calendar in iter {
        for e in calendar? {
            res.push(e);
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
    // TODO add timezone
    // calendar.add_timezone(TimeZone::new(
    //     "UTC",
    //     ZoneTime::standard("19700329T020000", "+0000", "+0000"),
    // ));
    // calendar.push(CalScale::new("GREGORIAN"));
    // calendar.push(Method::new("PUBLISH"));

    for e in events {
        let mut event = Event::new(e.uid, e.stamp);
        event.push(DtStart::new(e.start));
        event.push(DtEnd::new(e.end));
        event.push(Summary::new(e.summary));
        calendar.add_event(event);
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
        log::info!("starting up on {}", configuration.socketaddr);

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
