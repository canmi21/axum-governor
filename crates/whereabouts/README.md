# whereabouts

Where something is, answered from files on disk rather than by asking a service. Each lookup is a
feature, and none is on by default.

| Feature       | Answers                                                                 | Reads                                                                                                 |
| ------------- | ----------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `coordinates` | a latitude and longitude: country, region, city, postal code, time zone | GeoNames' `cities500.txt`, `countryInfo.txt`, `admin1CodesASCII.txt`, `admin2Codes.txt`, `postal.txt` |
| `ip`          | an IP address: country, region, city, position, time zone, network      | MaxMind's `GeoLite2-City.mmdb` and `GeoLite2-ASN.mmdb`                                                |

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

Fetching the data is the caller's: where it comes from, how often, and under which license. GeoLite2
asks anything built on it to credit MaxMind where the data is shown.
