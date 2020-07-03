# ical-filter
HTTP daemon that can parse any ical feed and represent it in a normalized fashion. Allows filtering of summaries.

**Note**: currently any other alarms, todos, journals and freebusies are not propagated. If you need this please file an issue.

**Note**: all datetimes are normalized to UTC.

## Setup

Build the binary and run with the following optional ENV variables:

```
RUST_LOG=info \
ICAL_FILTER_SOCKETADDR=127.0.0.1:8080 \
cargo run
```

## Endpoints

All endpoints accept the following query parameters:

* `url`: source url for an ical feed.
* `filter` *(optional)*: whitelist filter for summary field of events. Can only set one (for now).

### JSON
Retrieve the `url` and render as JSON with `Content-Type: application/json`.
```
http://localhost:8080/v1/json
```

Will retrieve similar to:
```json
[
    {
        "uid": "TU586226199",
        "summary": "reservation",
        "stamp": "2020-07-03T08:55:14Z",
        "created": "2020-01-20T16:17:34Z",
        "start": "2020-01-25T08:00:00Z",
        "end": "2020-01-26T08:00:00Z"
    },
    {
        "uid": "TU597147530",
        "summary": "reservation",
        "stamp": "2020-07-03T08:55:14Z",
        "created": "2020-01-28T07:45:28Z",
        "start": "2020-02-02T08:00:00Z",
        "end": "2020-02-03T08:00:00Z"
    }
]
```


### iCal
Retrieve the `url` and render as an iCalender v2.0 file with `Content-Type: text/calendar`.
```
http://localhost:8080/v1/ical
```

Will retrieve similar to:
```ical
BEGIN:VCALENDAR
VERSION:2.0
PRODID:ical-filter
CALSCALE:GREGORIAN
METHOD:PUBLISH
BEGIN:VTIMEZONE
TZID:UTC
BEGIN:STANDARD
DTSTART:19700329T020000
TZOFFSETFROM:+0000
TZOFFSETTO:+0000
END:STANDARD
END:VTIMEZONE
BEGIN:VEVENT
UID:TU586226199
DTSTAMP:20200703T085514Z
SUMMARY:Wouter
DTSTART:20200125T080000Z
DTEND:20200125T080000Z
CREATED:20200120T161734Z
END:VEVENT
BEGIN:VEVENT
UID:TU597147530
DTSTAMP:20200703T085514Z
SUMMARY:Wouter
DTSTART:20200202T080000Z
DTEND:20200202T080000Z
CREATED:20200128T074528Z
END:VEVENT
END:VCALENDAR
```