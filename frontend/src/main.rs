mod images;
mod zfs_unlocker;

use std::{str::FromStr, sync::LazyLock};

use common::{api::mock::ApiMock, config::WebPageConfig};
use leptos::*;
use zfs_unlocker::App;

const CONFIG_STR: &str = include_str!("../web.toml");

const CONFIG: LazyLock<WebPageConfig> = LazyLock::new(|| {
    WebPageConfig::from_str(CONFIG_STR)
        .unwrap_or_else(|e| panic!("Failed to find config file. Error: {e}"))
});

fn make_api() -> ApiMock {
    // leptos_dom::logging::console_log("Log something");

    match &CONFIG.mode {
        common::config::LiveOrMock::Live(_) => todo!(),
        common::config::LiveOrMock::Mock(m) => ApiMock::new_from_config(m.clone()),
    }
}

fn main() {
    console_error_panic_hook::set_once();

    let api_impl = make_api();

    mount_to_body(|| view! { <App api=api_impl /> })
}
