use dioxus::prelude::*;

#[component]
pub fn About() -> Element {
    rsx! {
        div {
            h2 { "Data source" }
            // or https://ctb-siri.s3.eu-south-2.amazonaws.com/metro-bilbao-trip-updates.xml if the day
            // comes
            p { "All data is sourced from the OPEN DATA Initiative at: "
                a {
                    href: "https://www.metrobilbao.eus/en/open-data/dataset/api-guide",
                    "Metro Bilbao API"
                }
                "."
            }
        }
        div {
            h2 { "Source Code" }
            p { "The source code is available at: "
                a { href:"https://github.com/xnacly/iritsi", "github.com/xnacly/iritsi" }
            }
        }
        div {
            p { "Made with love by " a { href:"https://xnacly.me", "xnacly" } }
        }
    }
}
