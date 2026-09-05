# Iritsi

<p align="center">
  <img width="256" height="256" src="https://raw.githubusercontent.com/xnacly/iritsi/refs/heads/master/iritsi-android/assets/icons/128x128.png"/>
</p>
 
An offline first bilbao public transportation android application.

## Features:

- Search for and mark stations as favorites
- View departures and arrivals for each station
- Plan, track and favorite travels
- Live notifications with
  - previous, current and next station
  - estimated arrival
  - direction
  - starting time
  - progress bra
- Handmade pixel art for all icons, logo and art


## Local setup

See [(Dioxus) Android Dev Setup for Arch
Linux](https://xnacly.me/posts/2026/dioxus-android-setup-arch-linux/) or use
nix develop with the provided flake.

## Project layout

- iritsi-core is the non ui logic for managing the underlying gtfs data storage, the route planning and other application logic
- iritsi-gtfs handles mobile optimised gtfs container decoding, parsing and zero allocation data preparation for consumption by core
- iritsi-android is the dioxus android application consuming the core api and rendering to the user
