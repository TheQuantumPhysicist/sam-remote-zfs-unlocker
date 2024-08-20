mod images;
mod zfs_unlocker;

use std::{str::FromStr, sync::LazyLock};

use common::{
    api::{mock::ApiMock, routed::ApiRouteImpl},
    config::WebPageConfig,
};
use leptos::*;
use zfs_unlocker::App;

const CONFIG_STR: &str = include_str!("../web.toml");

const CONFIG: LazyLock<WebPageConfig> = LazyLock::new(|| {
    WebPageConfig::from_str(CONFIG_STR)
        .unwrap_or_else(|e| panic!("Failed to find config file. Error: {e}"))
});

fn main() {
    console_error_panic_hook::set_once();

    // leptos_dom::logging::console_log("Log something");

    match CONFIG.mode.clone() {
        common::config::LiveOrMock::Live(s) => {
            mount_to_body(move || view! { <App api=ApiRouteImpl::new_from_config(s.clone()) /> });
        }
        common::config::LiveOrMock::Mock(m) => {
            mount_to_body(move || view! { <App api=ApiMock::new_from_config(m.clone()) /> });
        }
    }
}
