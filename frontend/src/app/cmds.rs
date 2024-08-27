use common::api::{mock::ApiMock, routed::ApiRouteImpl, traits::ZfsRemoteHighLevel};
use leptos::{component, create_local_resource, view, ErrorBoundary, IntoView, Transition};

use crate::{
    app::{config_reader::retrieve_config, error_fallback, log},
    images::RandomLoadingImage,
};

#[component]
pub fn CommandsTableFromConfig() -> impl IntoView {
    let configuration_getter =
        create_local_resource(|| (), move |_| async { retrieve_config().await });

    let after_config_view = move || {
        configuration_getter.and_then(|config| match config.mode.clone() {
            common::config::LiveOrMock::Live(s) => {
                log("Initializing live object for commands table");
                view! { <CommandsTable api=ApiRouteImpl::new_from_config(s) /> }
            }
            common::config::LiveOrMock::Mock(m) => {
                log("Initializing mock object for commands table");
                view! { <CommandsTable api=ApiMock::new_from_config(m) /> }
            }
        })
    };

    view! {
        <ErrorBoundary fallback=error_fallback>
            <Transition fallback=move || {
                view! {
                    <div class="config-loading-page">
                        <RandomLoadingImage />
                    </div>
                }
            }>
                <div>{after_config_view}</div>
            </Transition>
        </ErrorBoundary>
    }
}

#[component]
pub fn CommandsTable<A: ZfsRemoteHighLevel + 'static>(api: A) -> impl IntoView {
    let _api = api;
    view! { <p>"Commands table to go here"</p> }
}
