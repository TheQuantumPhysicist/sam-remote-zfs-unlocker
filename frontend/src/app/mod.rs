mod cmds;
mod config_reader;
mod dataset_state_retriever;
mod zfs;

use cmds::CommandsTableFromConfig;
use leptos::{component, view, CollectView, Errors, IntoView, RwSignal, SignalWith};
use zfs::ZfsTableFromConfig;

fn log(entry: &str) {
    leptos::leptos_dom::logging::console_log(entry);
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <ZfsTableFromConfig />
        <br />
        <CommandsTableFromConfig />
    }
}

fn error_fallback(errors: RwSignal<Errors>) -> impl IntoView {
    let error_list = move || {
        errors.with(|errors| {
            errors
                .iter()
                .map(|(_, e)| view! { <li>{e.to_string()}</li> })
                .collect_view()
        })
    };

    view! {
        <div class="error">
            <h2>"Error"</h2>
            <ul>{error_list}</ul>
        </div>
    }
}
