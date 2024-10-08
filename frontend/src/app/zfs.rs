use common::{
    api::traits::{ZfsRemoteAPI, ZfsRemoteHighLevel},
    types::{DatasetFullMountState, DatasetsFullMountState},
};
use leptos::{
    component, create_action, create_local_resource, create_signal, event_target_value, view,
    CollectView, ErrorBoundary, IntoView, Show, SignalGet, SignalSet, Transition,
};

use crate::{
    app::{error_fallback, log},
    images::RandomLoadingImage,
};

use super::dataset_state_retriever::DatasetStateResource;

async fn zfs_table_initial_query<A: ZfsRemoteHighLevel + 'static>(
    api: A,
) -> Result<(A, DatasetsFullMountState), A::Error> {
    let result = api.encrypted_datasets_state().await;

    result.map(|r| (api, r))
}

#[component]
pub fn ZfsUnlockTable<A: ZfsRemoteHighLevel + 'static>(api: A) -> impl IntoView {
    log("Creating ZFS table");

    let zfs_rows = create_local_resource(
        || (),
        move |_| {
            let api = api.clone();
            async move { zfs_table_initial_query(api).await }
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
                    <div class="zfs-loading-page">
                        <RandomLoadingImage />
                    </div>
                }
            }>
                <div>{zfs_table_view}</div>
            </Transition>
        </ErrorBoundary>
    }
}

#[component]
fn ZfsMountInput<A: ZfsRemoteHighLevel + 'static>(
    dataset_state_resource: DatasetStateResource<A>,
) -> impl IntoView {
    let dataset_name_for_mount = dataset_state_resource.dataset_name().to_string();

    let dataset_state_resource_for_action = dataset_state_resource.clone();
    // This action takes the action from the user, the click, and sends it to the API to unlock the dataset
    let mount_dataset = create_action(move |_: &()| {
        let mut api_for_mount = dataset_state_resource_for_action.api().clone();
        let dataset_name = dataset_name_for_mount.clone();
        let dataset_state_resource = dataset_state_resource_for_action.clone();
        async move {
            // We reset first, to trigger the loading animation
            dataset_state_resource.reset_dataset_state();
            let mount_result = api_for_mount.mount_dataset(&dataset_name).await;
            match mount_result {
                Ok(_) => log("Mount success"),
                Err(e) => log(&format!("Mount error: {e}")),
            }
            dataset_state_resource.refresh_dataset_state()
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
        let ds_info = dataset_state_resource.get();
        match ds_info {
            Some(key_loaded) => mount_field_or_already_mounted(key_loaded).into_view(),
            None => view! { <RandomLoadingImage /> }.into_view(),
        }
    }
}

#[component]
fn ZfsRefreshInput<A: ZfsRemoteHighLevel + 'static>(
    dataset_state_resource: DatasetStateResource<A>,
) -> impl IntoView {
    move || {
        let dataset_state_resource = dataset_state_resource.clone();
        view! {
            <button on:click=move |_| {
                dataset_state_resource.reset_dataset_state();
                dataset_state_resource.refresh_dataset_state();
            }>"Refresh"</button>
        }
        .into_view()
    }
}

#[component]
fn ZfsKeyPasswordInput<A: ZfsRemoteHighLevel + 'static>(
    dataset_state_resource: DatasetStateResource<A>,
) -> impl IntoView {
    let dataset_name_for_pw = dataset_state_resource.dataset_name().to_string();

    let api_for_pw = dataset_state_resource.api().clone();

    let (password_in_input, set_password_in_input) = create_signal("".to_string());

    let dataset_state_resource_for_action = dataset_state_resource.clone();

    // This action takes the action from the user, the click, and sends it to the API to unlock the dataset
    let load_key_password = create_action(move |password: &String| {
        let password = password.clone();
        let mut api_for_pw = api_for_pw.clone();
        let dataset_name = dataset_name_for_pw.clone();
        let dataset_state_resource = dataset_state_resource_for_action.clone();
        async move {
            // We reset first, to trigger the loading animation
            dataset_state_resource.reset_dataset_state();
            let load_key_result = api_for_pw.load_key(&dataset_name, &password).await;
            match load_key_result {
                Ok(_) => log("Load key success"),
                Err(e) => log(&format!("Load key error: {e}")),
            }
            dataset_state_resource.refresh_dataset_state()
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
                                placeholder="Dataset password"
                                on:input=move |ev| {
                                    set_password_in_input.set(event_target_value(&ev));
                                }
                                prop:value=password_in_input
                            />
                            <button on:click=move |_| {
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
        let reloaded_dataset = dataset_state_resource.get();
        let ds_info = reloaded_dataset.map(|ds| ds.map(|m| m.key_loaded));
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

#[component]
fn ZfsDatasetTableCell<A: ZfsRemoteHighLevel + 'static>(
    dataset_state_resource: Option<DatasetStateResource<A>>,
    column: ZFSTableColumnDefinition,
) -> impl IntoView {
    match column {
        ZFSTableColumnDefinition::Name => match dataset_state_resource {
            Some(ds) => view! {
                <div class="table-cell-dataset-name">
                    <p>{ds.dataset_name().to_string()}</p>
                </div>
            }
            .into_view(),
            None => view! { <p>"Dataset name"</p> }.into_view(),
        },
        ZFSTableColumnDefinition::KeyLoadPassword => match dataset_state_resource {
            Some(ds) => view! { <ZfsKeyPasswordInput dataset_state_resource=ds /> }.into_view(),
            None => view! { <p>"Key load"</p> }.into_view(),
        },
        ZFSTableColumnDefinition::MountButton => match dataset_state_resource {
            Some(ds) => view! { <ZfsMountInput dataset_state_resource=ds /> }.into_view(),
            None => view! { <p>"Mount"</p> }.into_view(),
        },
        ZFSTableColumnDefinition::RefreshButton => match dataset_state_resource {
            Some(ds) => view! { <ZfsRefreshInput dataset_state_resource=ds /> }.into_view(),
            None => view! { <p>"Refresh"</p> }.into_view(),
        },
    }
}

#[allow(clippy::needless_lifetimes)]
#[component]
fn ZfsDatasetRow<'a, A: ZfsRemoteHighLevel + 'static>(
    api: A,
    initial_mount_state: Option<&'a DatasetFullMountState>,
) -> impl IntoView {
    let dataset_state_resource = initial_mount_state
        .as_ref()
        .map(|m| DatasetStateResource::new(m.dataset_name.to_string(), api, &log));

    view! {
        <tr>
            <th>
                <ZfsDatasetTableCell
                    dataset_state_resource=dataset_state_resource.clone()
                    column=ZFSTableColumnDefinition::Name
                />
            </th>
            <th>
                <ZfsDatasetTableCell
                    dataset_state_resource=dataset_state_resource.clone()
                    column=ZFSTableColumnDefinition::KeyLoadPassword
                />
            </th>
            <th>
                <ZfsDatasetTableCell
                    dataset_state_resource=dataset_state_resource.clone()
                    column=ZFSTableColumnDefinition::MountButton
                />
            </th>
            <th>
                <ZfsDatasetTableCell
                    dataset_state_resource=dataset_state_resource.clone()
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

    let datasets = (*unmounted_datasets).clone();

    view! {
        <div class="zfs-datasets-table-container">
            <Show when=move || (locked_count > 0) fallback=|| view! { <NothingToUnlock /> }>
                <table class="zfs-datasets-table">
                    <thead>
                        <ZfsDatasetRow api=api.clone() initial_mount_state=None />
                    </thead>
                    <tbody>
                        {datasets
                            .states
                            .values()
                            .map(|mount_data| {
                                view! {
                                    <ZfsDatasetRow
                                        api=api.clone()
                                        initial_mount_state=Some(mount_data)
                                    />
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </Show>
        </div>
    }
}

#[component]
fn NothingToUnlock() -> impl IntoView {
    view! { <p align="center">"No ZFS datasets to show"</p> }
}
