# Whereabouts

Offline place names and IP lookup, from [GeoNames](https://www.geonames.org) and
[GeoLite2](https://dev.maxmind.com/geoip/geolite2-free-geolocation-data) files on disk.

## Quick start

```toml
[dependencies]
whereabouts = { version = "1", features = ["coordinates", "ip"] }
```

```rust
use std::path::Path;
use whereabouts::{coordinates::Gazetteer, ip::Databases};

let gazetteer = Gazetteer::open(Path::new("data/geonames")).expect("GeoNames is fetched");
let place = gazetteer.lookup(64.15, -21.95);

// SAFETY: new files are renamed into place, never written over the mapped ones.
let databases = unsafe { Databases::open(Path::new("data/geolite2")) }?;
let location = databases.lookup("1.1.1.1".parse()?);
```

## Features

- **Reverse geocoding** — `Gazetteer::lookup` turns a latitude and longitude into the nearest
  place: country, region, city, district, postal code and time zone.
- **IP lookup** — `Databases::lookup` turns an address into its country, region, city, position,
  time zone and network.
- **Memory-mapped** — the GeoLite2 files are mapped, so the page cache holds what is asked and the
  heap nothing; `Databases::from_bytes` takes them from memory instead.
- **No network** — nothing is fetched. Where the data comes from, how often, and under which
  license is up to you.

## Cargo features

| Feature       | Default | Description                                         |
| ------------- | ------- | --------------------------------------------------- |
| `coordinates` | No      | Place names from GeoNames, `coordinates::Gazetteer` |
| `ip`          | No      | IP lookup from GeoLite2, `ip::Databases`            |

Each reads its own files from the directory it is given:

| Feature       | Reads                                                                                                 |
| ------------- | ----------------------------------------------------------------------------------------------------- |
| `coordinates` | GeoNames' `cities500.txt`, `countryInfo.txt`, `admin1CodesASCII.txt`, `admin2Codes.txt`, `postal.txt` |
| `ip`          | MaxMind's `GeoLite2-City.mmdb` and `GeoLite2-ASN.mmdb`                                                |

GeoLite2 asks anything built on it to credit MaxMind where the data is shown.

## License

Released under the MIT License © 2024 [Canmi](https://canmi.net)
