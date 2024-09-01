mod cmds;
mod command_communicator;
mod config_reader;
mod dataset_state_retriever;
mod modal;
mod zfs;

use cmds::CommandsTable;
use common::{
    api::{api_wrapper::ApiAny, mock::ApiMock, routed::ApiRouteImpl, traits::ZfsRemoteHighLevel},
    config::WebPageConfig,
};
use config_reader::retrieve_config;
use leptos::{
    component, create_local_resource, create_signal, view, CollectView, Errors, IntoView, RwSignal,
    SignalGet, SignalUpdate, SignalWith,
};
use zfs::ZfsUnlockTable;

use crate::images::RandomLoadingImage;

fn log(entry: &str) {
    leptos::leptos_dom::logging::console_log(entry);
}

#[component]
pub fn App() -> impl IntoView {
    view! { <InitComponent base_url=None /> }
}

#[component]
fn InitComponent(base_url: Option<String>) -> impl IntoView {
    let configuration_getter =
        create_local_resource(|| (), move |_| async { retrieve_config().await });

    let api_from_config_getter =
        move || configuration_getter.and_then(|config| api_from_config(config.clone()));

    let main_page_view = view! {
        {move || match api_from_config_getter() {
            Some(Ok(api)) => view! { <MainPage api=api.clone() /> }.into_view(),
            Some(Err(e)) => {
                view! {
                    <div class="config-load-error">
                        <p>{format!("Error loading config file.")}</p>
                        <ToggleText to_show=e.to_string() to_show_name="error".to_string() />
                        <hr />
                    </div>
                }
                    .into_view()
            }
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

    // Choose API from a given URL or load the info from a config file
    let api_chooser = move || match &base_url {
        Some(url) => {
            let api = api_from_config(WebPageConfig::from_base_url(url.clone()));
            view! { <MainPage api=api /> }.into_view()
        }
        None => main_page_view.into_view(),
    };

    api_chooser.into_view()
}

#[component]
fn MainPage<A: ZfsRemoteHighLevel + 'static>(api: A) -> impl IntoView {
    view! {
        <h3 align="center">"Custom commands"</h3>
        <CommandsTable api=api.clone() />
        <hr />
        <h3 align="center">"ZFS datasets"</h3>
        <ZfsUnlockTable api=api />
    }
}

fn api_from_config(config: WebPageConfig) -> ApiAny {
    match config.mode.clone() {
        common::config::LiveOrMock::Live(s) => {
            log("Initializing live object");
            ApiAny::Live(ApiRouteImpl::new_from_config(s))
        }
        common::config::LiveOrMock::Mock(m) => {
            log("Initializing mock object");
            ApiAny::Mock(ApiMock::new_from_config(m))
        }
    }
}

#[component]
fn ToggleText(to_show: String, to_show_name: String) -> impl IntoView {
    let (is_visible, set_visible) = create_signal(false);

    view! {
        <button on:click=move |_| {
            set_visible.update(|v| *v = !*v)
        }>
            {move || {
                if is_visible.get() {
                    format!("Hide {} ↓", to_show_name)
                } else {
                    format!("Show {} →", to_show_name)
                }
            }}
        </button>
        {move || {
            if is_visible.get() {
                view! { <p>{to_show.clone()}</p> }
            } else {
                view! { <p></p> }
            }
        }}
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
