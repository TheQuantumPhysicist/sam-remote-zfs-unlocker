mod cmds;
mod command_communicator;
mod config_reader;
mod dataset_state_retriever;
mod modal;
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
        <h3 align="center">"Custom commands"</h3>
        <CommandsTableFromConfig />
        <hr />
        <h3 align="center">"ZFS datasets"</h3>
        <ZfsTableFromConfig />
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
