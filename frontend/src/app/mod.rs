mod browser_helpers;
mod cmds;
mod command_communicator;
mod config_reader;
mod dataset_state_retriever;
mod modal;
mod zfs;

use browser_helpers::{get_value_from_storage, set_value_in_storage};
use cmds::CommandsTable;
use common::{
    api::{api_wrapper::ApiAny, mock::ApiMock, routed::ApiRouteImpl, traits::ZfsRemoteHighLevel},
    config::WebPageConfig,
};
use config_reader::retrieve_config;
use leptos::{
    component, create_local_resource, create_signal, event_target_value, view, CollectView, Errors,
    IntoView, RwSignal, SignalGet, SignalSet, SignalUpdate, SignalWith, WriteSignal,
};
use zfs::ZfsUnlockTable;

use crate::images::RandomLoadingImage;

fn log(entry: &str) {
    leptos::leptos_dom::logging::console_log(entry);
}

#[component]
pub fn App() -> impl IntoView {
    view! { <FullPage /> }
}

#[component]
fn NavBar(contents_page_setter: WriteSignal<leptos::View>) -> impl IntoView {
    let on_logout = move |_| {
        contents_page_setter.set(
            view! {
                <div class="login-dialog">
                    <EnterAPIAddress contents_page_setter />
                </div>
            }
            .into_view(),
        )
    };

    view! {
        <nav class="navbar">
            // inside "navbar-list-left" below, one can put menu items that will go to the left of the nav bar
            <ul class="navbar-list-left"></ul>
            <ul class="navbar-list-right">
                <li class="navbar-item">
                    <a href="#" on:click=on_logout>
                        "Logout"
                    </a>
                </li>
            </ul>
        </nav>
    }
}

#[component]
fn FullPage() -> impl IntoView {
    let (contents_page_getter, contents_page_setter) = create_signal(view! {}.into_view());

    contents_page_setter.set(view! { <ContentsPage base_url=None contents_page_setter /> });

    move || {
        view! {
            <NavBar contents_page_setter />
            {contents_page_getter.get()}
        }
    }
}

#[component]
fn ContentsPage(
    base_url: Option<String>,
    contents_page_setter: WriteSignal<leptos::View>,
) -> impl IntoView {
    let configuration_getter =
        create_local_resource(|| (), move |_| async { retrieve_config().await });

    let api_from_config_getter =
        move || configuration_getter.and_then(|config| api_from_config(config.clone()));

    let contents_page_on_config = {
        move || {
            view! {
                {match api_from_config_getter() {
                    Some(Ok(api)) => {
                        view! { <TablesPage api=api.clone() contents_page_setter /> }.into_view()
                    }
                    Some(Err(err)) => {
                        view! { <ConfigConnectError err contents_page_setter /> }.into_view()
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
            }
        }
    };

    let contents_page_on_base_url = move |url: &String| {
        let api = api_from_config(WebPageConfig::from_base_url(url));
        view! { <TablesPage api contents_page_setter /> }.into_view()
    };

    // Choose API from a given URL or load the info from a config file
    let contents_page_view = move || match base_url.clone() {
        Some(url) => { move || contents_page_on_base_url(&url) }.into_view(),
        None => contents_page_on_config.into_view(),
    };

    move || {
        contents_page_setter.set(contents_page_view.clone().into_view());
        contents_page_view()
    }
}

#[component]
fn EnterAPIAddress(contents_page_setter: WriteSignal<leptos::View>) -> impl IntoView {
    let ADDRESS_IN_STORAGE_KEY: &str = "last_ip_address";

    let (url_input, set_url_input) =
        create_signal(get_value_from_storage(ADDRESS_IN_STORAGE_KEY).unwrap_or_default());

    view! {
        <p>"Enter API URL or attempt to reload config file"</p>
        <input
            type="text"
            placeholder="https://..."
            on:input=move |ev| {
                set_url_input.set(event_target_value(&ev));
            }
            prop:value=url_input
        />
        <button on:click=move |_| {
            set_value_in_storage(ADDRESS_IN_STORAGE_KEY, url_input.get());
            contents_page_setter
                .set(
                    view! { <ContentsPage base_url=Some(url_input.get()) contents_page_setter /> }
                        .into_view(),
                );
        }>"Connect"</button>
        <button on:click=move |_| {
            contents_page_setter
                .set(view! { <ContentsPage base_url=None contents_page_setter /> }.into_view());
        }>"Load config file"</button>
    }
}

#[component]
fn TablesPage<A: ZfsRemoteHighLevel + 'static>(
    api: A,
    contents_page_setter: WriteSignal<leptos::View>,
) -> impl IntoView {
    let api_for_tester = api.clone();
    let api_tester = create_local_resource(
        || (),
        move |_| {
            let api = api_for_tester.clone();
            async move { api.clone().test_connection().await }
        },
    );

    let main_page_view = view! {
        {move || match api_tester.get() {
            Some(Ok(_)) => {
                view! {
                    <h3 align="center">"Custom commands"</h3>
                    <CommandsTable api=api.clone() />
                    <hr />
                    <h3 align="center">"ZFS datasets"</h3>
                    <ZfsUnlockTable api=api.clone() />
                }
                    .into_view()
            }
            Some(Err(err)) => {
                view! { <ConfigConnectError err contents_page_setter /> }
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

    main_page_view.into_view()
}

#[component]
fn ConfigConnectError(
    err: impl std::error::Error,
    contents_page_setter: WriteSignal<leptos::View>,
) -> impl IntoView {
    view! {
        <div class="config-load-error">
            <p>"Error loading config file."</p>
            <ToggleText to_show=err.to_string() to_show_name="error".to_string() />
            <hr />
            <EnterAPIAddress contents_page_setter />
        </div>
    }
    .into_view()
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
