use std::{str::FromStr, sync::Arc};

use crate::images::RandomLoadingImage;
use common::{
    api::{
        api_wrapper::ApiAny,
        mock::ApiMock,
        routed::ApiRouteImpl,
        traits::{ZfsRemoteAPI, ZfsRemoteHighLevel},
    },
    config::WebPageConfig,
    types::{DatasetFullMountState, DatasetsFullMountState},
};
use futures::FutureExt;
use leptos::{
    component, create_action, create_local_resource, create_signal, event_target_value, view,
    CollectView, ErrorBoundary, Errors, IntoView, Resource, RwSignal, Show, SignalGet, SignalSet,
    SignalWith, Transition,
};

const CONFIG_URL: &str = "/public/web.toml";

#[derive(thiserror::Error, Debug, Clone)]
pub enum ConfigurationLoadError {
    #[error("Configuration retrieval error. Configuration is expected to be found in path {1}. Error: {0}")]
    Retrieval(String, String),
    #[error("Failed to get configuration file as text to parse. Error: {0}")]
    TextRetrieval(String),
    #[error("Failed to parse config file: {0}")]
    FileParse(String),
}

async fn initial_table_query<A: ZfsRemoteHighLevel + 'static>(
    api: A,
) -> Result<(A, DatasetsFullMountState), A::Error> {
    let result = api.encrypted_datasets_state().await;

    result.map(|r| (api, r))
}

async fn retrieve_config() -> Result<WebPageConfig, ConfigurationLoadError> {
    let url = CONFIG_URL;

    let config_file = reqwasm::http::Request::get(url)
        .send()
        .await
        .map_err(|e| ConfigurationLoadError::Retrieval(e.to_string(), url.to_string()))?
        // convert it to JSON
        .text()
        .await
        .map_err(|e| ConfigurationLoadError::TextRetrieval(e.to_string()))?;

    let webpage_config = WebPageConfig::from_str(&config_file)
        .map_err(|e| ConfigurationLoadError::FileParse(e.to_string()))?;
    Ok(webpage_config)
}

#[component]
pub fn App() -> impl IntoView {
    let configuration_getter =
        create_local_resource(|| (), move |_| async move { retrieve_config().await });

    move || {
        configuration_getter.and_then(|config| {
            let api: ApiAny = match config.mode.clone() {
                common::config::LiveOrMock::Live(s) => {
                    ApiRouteImpl::new_from_config(s.clone()).into()
                }
                common::config::LiveOrMock::Mock(m) => ApiMock::new_from_config(m.clone()).into(),
            };

            view! {
                <Transition fallback=move || {
                    view! {
                        <div class="config-loading-page">
                            <RandomLoadingImage />
                        </div>
                    }
                }>
                    <div>
                        <ZfsUnlockTable api=api.clone() />
                    </div>
                </Transition>
            }
        })
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

#[component]
pub fn ZfsUnlockTable<A: ZfsRemoteHighLevel + 'static>(api: A) -> impl IntoView {
    let zfs_rows = create_local_resource(
        || (),
        move |_| {
            let api = api.clone();
            async move { initial_table_query(api).await }
        },
    );

    let zfs_table_view = move || {
        zfs_rows.and_then(|(api, rows)| {
            view! { <ZfsUnlocksTable api=api.clone() unmounted_datasets=rows /> }
        })
    };

    view! {
        <ErrorBoundary fallback=error_fallback>
            <Transition fallback=move || {
                view! {
                    <div class="first-loading-page">
                        <RandomLoadingImage />
                    </div>
                }
            }>
                <div>{zfs_table_view}</div>
            </Transition>
        </ErrorBoundary>
    }
}

#[allow(clippy::needless_lifetimes)]
#[component]
fn ZfsMountInput<'a, A: ZfsRemoteHighLevel + 'static>(
    api: A,
    current_mount_state: &'a DatasetFullMountState,
    dataset_state_resource: Resource<
        (),
        Option<Result<DatasetFullMountState, <A as ZfsRemoteAPI>::Error>>,
    >,
) -> impl IntoView {
    let dataset_name = Arc::new(current_mount_state.dataset_name.to_string());
    let dataset_name_for_mount = dataset_name.clone();

    // This action takes the action from the user, the click, and sends it to the API to unlock the dataset
    let mount_dataset = create_action(move |_: &()| {
        let mut api_for_mount: A = api.clone();
        let dataset_name = dataset_name_for_mount.clone();
        async move {
            let _mount_result = api_for_mount.mount_dataset(&dataset_name).await;
            dataset_state_resource.refetch()
        }
    });

    // This contains the text field + submit button objects, depending on whether the key is loaded or not
    let mount_field_or_already_mounted =
        move |mount_state: Result<DatasetFullMountState, <A as ZfsRemoteAPI>::Error>| {
            match mount_state {
            Ok(state) => view! {
                <Show when=move || state.key_loaded fallback=|| view! { "Load key first" }>
                    <Show when=move || !state.is_mounted fallback=|| view! { "Dataset is mounted" }>
                        {
                            view! {
                                <button on:click=move |_| {
                                    dataset_state_resource.set(None);
                                    mount_dataset.dispatch(());
                                }>"Mount dataset"</button>
                            }
                        }
                    </Show>
                </Show>
            }
            .into_view(),
            Err(e) => view! {
                "Key loading error: "
                {e.to_string()}
            }
            .into_view(),
        }
        };

    move || {
        // We flatten because we have 2 Option wraps:
        // 1. The Option from create_local_resource finishing
        // 2. The Option that we manually added, so that we set it to None when the user clicks on "Submit"
        let ds_info = dataset_state_resource.get().flatten().clone();
        match ds_info {
            Some(key_loaded) => mount_field_or_already_mounted(key_loaded).into_view(),
            None => view! { <RandomLoadingImage /> }.into_view(),
        }
    }
}

#[allow(clippy::needless_lifetimes)]
#[component]
fn ZfsRefreshInput<A: ZfsRemoteHighLevel + 'static>(
    api: A,
    dataset_state_resource: Resource<
        (),
        Option<Result<DatasetFullMountState, <A as ZfsRemoteAPI>::Error>>,
    >,
) -> impl IntoView {
    let _api = api;

    move || {
        view! {
            <button on:click=move |_| {
                dataset_state_resource.set(None);
                dataset_state_resource.refetch();
            }>"Refresh"</button>
        }
    }
}

#[allow(clippy::needless_lifetimes)]
#[component]
fn ZfsKeyPasswordInput<'a, A: ZfsRemoteHighLevel + 'static>(
    api: A,
    current_mount_state: &'a DatasetFullMountState,
    dataset_state_resource: Resource<
        (),
        Option<Result<DatasetFullMountState, <A as ZfsRemoteAPI>::Error>>,
    >,
) -> impl IntoView {
    let dataset_name = Arc::new(current_mount_state.dataset_name.to_string());
    let dataset_name_for_pw = dataset_name.clone();

    let api_for_pw: A = api.clone();

    let (password_in_input, set_password_in_input) = create_signal("".to_string());

    // This action takes the action from the user, the click, and sends it to the API to unlock the dataset
    let load_key_password = create_action(move |password: &String| {
        let password = password.clone();
        let mut api_for_pw: A = api_for_pw.clone();
        let dataset_name = dataset_name_for_pw.clone();
        async move {
            let _load_key_result = api_for_pw.load_key(&dataset_name, &password).await;
            dataset_state_resource.refetch()
        }
    });

    // This contains the text field + submit button objects, depending on whether the key is loaded or not
    let password_field_or_key_already_loaded = move |key_loaded_result: Result<
        bool,
        <A as ZfsRemoteAPI>::Error,
    >| {
        match key_loaded_result {
            Ok(key_loaded) => view! {
                <Show when=move || !key_loaded fallback=|| view! { "Key loaded" }>
                    {
                        view! {
                            <input
                                type="password"
                                on:input=move |ev| {
                                    set_password_in_input.set(event_target_value(&ev));
                                }
                                prop:value=password_in_input
                            />
                            <button on:click=move |_| {
                                dataset_state_resource.set(None);
                                load_key_password.dispatch(password_in_input.get());
                            }>"Load key"</button>
                        }
                    }
                </Show>
            }
            .into_view(),
            Err(e) => view! {
                "Key loading error: "
                {e.to_string()}
            }
            .into_view(),
        }
    };

    move || {
        // We flatten because we have 2 Option wraps:
        // 1. The Option from create_local_resource finishing
        // 2. The Option that we manually added, so that we set it to None when the user clicks on "Submit"
        let reloaded_dataset = dataset_state_resource.get().flatten();
        let ds_info = reloaded_dataset.map(|ds| ds.clone().map(|m| m.key_loaded));
        match ds_info {
            Some(key_loaded) => password_field_or_key_already_loaded(key_loaded).into_view(),
            None => view! { <RandomLoadingImage /> }.into_view(),
        }
    }
}

enum ZFSTableColumnDefinition {
    Name,
    KeyLoadPassword,
    MountButton,
    RefreshButton,
}

#[allow(clippy::needless_lifetimes)]
#[component]
fn ZfsDatasetTableCell<'a, A: ZfsRemoteHighLevel + 'static>(
    api: A,
    current_mount_state: Option<&'a DatasetFullMountState>,
    dataset_state_resource: Resource<
        (),
        Option<Result<DatasetFullMountState, <A as ZfsRemoteAPI>::Error>>,
    >,
    column: ZFSTableColumnDefinition,
) -> impl IntoView {
    let api_for_pw = api.clone();
    let api_for_mount = api.clone();

    match column {
    ZFSTableColumnDefinition::Name => match current_mount_state {
        Some(r) => view! {
            <div class="table-cell-dataset-name">
                <p>{r.dataset_name.to_string()}</p>
            </div>
        }.into_view(),
        None => view! { <p>"Dataset name"</p> }.into_view()
    },
    ZFSTableColumnDefinition::KeyLoadPassword => match current_mount_state {
        Some(r) => view! { <ZfsKeyPasswordInput api=api_for_pw current_mount_state=r dataset_state_resource /> }.into_view(),
        None => view! { <p>"Key load"</p> }.into_view()
    },
    ZFSTableColumnDefinition::MountButton => match current_mount_state {
        Some(r) => view! { <ZfsMountInput api=api_for_mount current_mount_state=r dataset_state_resource /> }.into_view(),
        None => view! { <p>"Mount"</p> }.into_view()
    }
    ZFSTableColumnDefinition::RefreshButton => match current_mount_state {
        Some(_) => view! { <ZfsRefreshInput api=api_for_mount dataset_state_resource /> }.into_view(),
        None => view! { <p>"Refresh"</p> }.into_view()
    },
}
}

#[allow(clippy::needless_lifetimes)]
#[component]
fn ZfsDatasetRow<'a, A: ZfsRemoteHighLevel + 'static>(
    api: A,
    current_mount_state: Option<&'a DatasetFullMountState>,
) -> impl IntoView {
    let dataset_name = current_mount_state
        .as_ref()
        .map(|m| m.dataset_name.to_string())
        .unwrap_or("".to_string());
    let api_for_name = api.clone();
    let api_for_pw = api.clone();
    let api_for_mount = api.clone();
    let api_for_refresh = api.clone();

    let dataset_state_resource = create_local_resource(
        move || (),
        move |_| {
            let api = api.clone();
            let dataset_name = dataset_name.clone();
            // We wrap with Some, because None is used to trigger reloading after the user submits the password
            async move { api.encrypted_dataset_state(&dataset_name).map(Some).await }
        },
    );

    view! {
        <tr>
            <th>
                <ZfsDatasetTableCell
                    api=api_for_name
                    current_mount_state
                    dataset_state_resource
                    column=ZFSTableColumnDefinition::Name
                />
            </th>
            <th>
                <ZfsDatasetTableCell
                    api=api_for_pw
                    current_mount_state
                    dataset_state_resource
                    column=ZFSTableColumnDefinition::KeyLoadPassword
                />
            </th>
            <th>
                <ZfsDatasetTableCell
                    api=api_for_mount
                    current_mount_state
                    dataset_state_resource
                    column=ZFSTableColumnDefinition::MountButton
                />
            </th>
            <th>
                <ZfsDatasetTableCell
                    api=api_for_refresh
                    current_mount_state
                    dataset_state_resource
                    column=ZFSTableColumnDefinition::RefreshButton
                />
            </th>
        </tr>
    }
}

#[allow(clippy::needless_lifetimes)]
#[component]
fn ZfsUnlocksTable<'a, A: ZfsRemoteHighLevel + 'static>(
    api: A,
    unmounted_datasets: &'a DatasetsFullMountState,
) -> impl IntoView {
    let locked_count = unmounted_datasets.states.len();

    let unmounted_datasets = (*unmounted_datasets).clone();

    view! {
        <div class="zfs-datasets-table-container">
            <table class="zfs-datasets-table">
                <thead>
                    <ZfsDatasetRow api=api.clone() current_mount_state=None />
                </thead>
                <tbody>
                    <Show when=move || (locked_count > 0) fallback=|| view! { <NothingToUnlock /> }>
                        {unmounted_datasets
                            .states
                            .values()
                            .map(|mount_data| {
                                view! {
                                    <ZfsDatasetRow
                                        api=api.clone()
                                        current_mount_state=Some(mount_data)
                                    />
                                }
                            })
                            .collect_view()}
                    </Show>
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn NothingToUnlock() -> impl IntoView {
    view! { <p>"All datasets unlocked and mounted"</p> }
}
