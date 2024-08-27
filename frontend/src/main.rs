mod app;
mod images;

use app::App;
use leptos::*;

fn main() {
    console_error_panic_hook::set_once();

    mount_to_body(move || view! { <App /> });
}
