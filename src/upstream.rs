use crate::Result;
use actix_web::client::ClientBuilder;
use ical::parser::ical::{component::IcalCalendar, IcalParser};

pub async fn get_calendars(url: &str) -> Result<impl Iterator<Item = Result<IcalCalendar>>> {
    let client = ClientBuilder::new()
        .no_default_headers()
        .header("User-Agent", "ical-filter")
        .finish();

    use bytes::buf::ext::BufExt;
    let buf = client.get(url).send().await?.body().await?.reader();

    let reader = std::io::BufReader::new(buf);
    Ok(IcalParser::new(reader).map(|calendar| calendar.map_err(|e| e.into())))
}
