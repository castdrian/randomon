macro_rules! patch_table {
    ($($name:literal),+ $(,)?) => {
        pub fn patch_bytes(name: &str) -> Option<&'static [u8]> {
            match name {
                $($name => Some(include_bytes!(concat!("../assets/patches/", $name, ".ips"))),)+
                _ => None,
            }
        }
    };
}

patch_table!(
    "instant_text/b1_instant_text",
    "instant_text/b2_instant_text",
    "instant_text/w1_instant_text",
    "instant_text/w2_instant_text",
    "musicfix/black2_musicfix",
    "musicfix/black2_ovl36_musicfix",
    "musicfix/black_musicfix",
    "musicfix/black_ovl21_musicfix",
    "musicfix/white2_musicfix",
    "musicfix/white2_ovl36_musicfix",
    "musicfix/white_musicfix",
    "musicfix/white_ovl21_musicfix",
    "national_dex/bw1_national_dex",
    "national_dex/bw2_national_dex",
    "shedinja/black2_ovl284_shedinja",
    "shedinja/black2_shedinja",
    "shedinja/black_ovl195_shedinja",
    "shedinja/black_shedinja",
    "shedinja/white2_ovl284_shedinja",
    "shedinja/white2_shedinja",
    "shedinja/white_ovl195_shedinja",
    "shedinja/white_shedinja",
);
