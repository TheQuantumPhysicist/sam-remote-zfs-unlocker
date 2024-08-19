use std::sync::LazyLock;

use base64::prelude::*;
use leptos::{component, view, IntoView};
use rand::Rng;

const SPINNING_ANIMS_DATA: [&[u8]; 9] = [
    include_bytes!("../public/3DSnake.gif"),
    include_bytes!("../public/BallInBowl.gif"),
    include_bytes!("../public/Book.gif"),
    include_bytes!("../public/Circles-menu-3.gif"),
    include_bytes!("../public/Fidget-spinner.gif"),
    include_bytes!("../public/Radar.gif"),
    include_bytes!("../public/Rhombus.gif"),
    include_bytes!("../public/Rocket.gif"),
    include_bytes!("../public/Spinner-2.gif"),
];

const SPINNING_ANIMS_BASE64: LazyLock<Vec<String>> = LazyLock::new(|| {
    SPINNING_ANIMS_DATA
        .into_iter()
        .map(|d| BASE64_STANDARD.encode(d))
        .collect()
});

#[component]
pub fn RandomLoadingImage() -> impl IntoView {
    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..SPINNING_ANIMS_BASE64.len());
    let image_base64 = &SPINNING_ANIMS_BASE64[index];
    view! { <img src=format!("data:image/png;base64,{}", image_base64) alt="Loading..." /> }
}
