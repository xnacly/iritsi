pub struct License {
    pub name: &'static str,
    pub kind: &'static str,
    pub url: &'static str,
    pub text: &'static str,
}

pub static LICENSES: &[License] = &[
    License {
        name: "Dioxus",
        kind: "MIT",
        url: "https://github.com/DioxusLabs/dioxus",
        text: include_str!("../licenses/dioxus.txt"),
    },
    License {
        name: "Tokio",
        kind: "MIT",
        url: "https://github.com/tokio-rs/tokio",
        text: include_str!("../licenses/tokio.txt"),
    },
    License {
        name: "rusqlite",
        kind: "MIT",
        url: "https://github.com/rusqlite/rusqlite",
        text: include_str!("../licenses/rusqlite.txt"),
    },
    License {
        name: "SQLite",
        kind: "Public Domain",
        url: "https://sqlite.org/copyright.html",
        text: include_str!("../licenses/sqlite.txt"),
    },
    License {
        name: "ureq",
        kind: "MIT",
        url: "https://github.com/algesten/ureq",
        text: include_str!("../licenses/ureq.txt"),
    },
    License {
        name: "csv",
        kind: "MIT",
        url: "https://github.com/BurntSushi/rust-csv",
        text: include_str!("../licenses/csv.txt"),
    },
    License {
        name: "zip",
        kind: "MIT",
        url: "https://github.com/zip-rs/zip2",
        text: include_str!("../licenses/zip.txt"),
    },
    License {
        name: "Departure Mono",
        kind: "OFL-1.1",
        url: "https://departuremono.com",
        text: include_str!("../licenses/departure-mono.txt"),
    },
    License {
        name: "Maple Mono",
        kind: "OFL-1.1",
        url: "https://github.com/subframe7536/maple-font",
        text: include_str!("../licenses/maple-mono.txt"),
    },
];
