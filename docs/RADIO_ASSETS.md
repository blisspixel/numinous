# Radio assets

Music is core to Numinous. The source tree ships the programmatic engine and the
recorded station soundtrack. Nick Seal made the music specifically for Numinous,
and it is part of the game experience.

## Pack contents

The repository contains 42 high-quality V0 MP3 recordings across three stations:

- 14 `trance` tracks for NUMINA FM;
- 14 `chill` tracks for THE ATTRACTOR;
- 14 `arcade` tracks for EIGHT BIT SUNRISE.

The compressed recordings total 268,973,810 bytes and 151.4 minutes. Filenames match the
station identifiers in `crates/core/src/radio.rs` and the cache discovery rules
in `faces/app/src/radio_cache.rs`.

## License

The recordings are made available under the
[Creative Commons CC0 1.0 Universal public-domain dedication](https://creativecommons.org/publicdomain/zero/1.0/).
To the fullest extent permitted by law, the project rights holder waives
copyright and related rights in these recordings worldwide. No attribution is
required.

## Playback

The app finds `assets/radio` automatically. In the app, Y tunes through the
stations. `NUMINOUS_RADIO` can point to a different compatible MP3 or WAV pack
for development.
