mod cmds;
mod command_communicator;
mod config_reader;
mod dataset_state_retriever;
mod modal;
mod zfs;

use cmds::CommandsTable;
use common::api::{api_wrapper::ApiAny, mock::ApiMock, routed::ApiRouteImpl};
use config_reader::retrieve_config;
use leptos::{
    component, create_local_resource, view, CollectView, Errors, IntoView, RwSignal, SignalWith,
};
use zfs::ZfsUnlockTable;

use crate::images::RandomLoadingImage;

fn log(entry: &str) {
    leptos::leptos_dom::logging::console_log(entry);
}

#[component]
pub fn App() -> impl IntoView {
    let configuration_getter =
        create_local_resource(|| (), move |_| async { retrieve_config().await });

    let api_getter = move || {
        configuration_getter.and_then(|config| match config.mode.clone() {
            common::config::LiveOrMock::Live(s) => {
                log("Initializing live object");
                ApiAny::Live(ApiRouteImpl::new_from_config(s))
            }
            common::config::LiveOrMock::Mock(m) => {
                log("Initializing mock object");
                ApiAny::Mock(ApiMock::new_from_config(m))
            }
        })
    };

    let main_page_view = view! {
        {move || match api_getter() {
            Some(Ok(config)) => {
                view! {
                    <h3 align="center">"Custom commands"</h3>
                    <CommandsTable api=config.clone() />
                    <hr />
                    <h3 align="center">"ZFS datasets"</h3>
                    <ZfsUnlockTable api=config />
                }
                    .into_view()
            }
            Some(Err(e)) => view! { <p>{format!("Error loading config {e}")}</p> }.into_view(),
            None => {
                view! {
                    <div class="config-loading-page">
                        <RandomLoadingImage />
                    </div>
                }
                    .into_view()
            }
        }}
    };

    main_page_view.into_view()
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
