mod images;
mod zfs_unlocker;

use leptos::*;
use zfs_unlocker::App;

fn main() {
    console_error_panic_hook::set_once();

    // leptos_dom::logging::console_log("Log something");

    mount_to_body(move || view! { <App /> });
}
