use std::sync::Arc;

use common::{
    api::{
        mock::ApiMock,
        traits::{ZfsRemoteAPI, ZfsRemoteHighLevel},
    },
    types::{DatasetFullMountState, DatasetsFullMountState},
};
use futures::{join, FutureExt};
use leptos::*;

fn make_mock() -> ApiMock {
    ApiMock::new(
        true,
        vec![
            (
                "MyPool/MyFirstDataset".to_string(),
                "my-password".to_string(),
            ),
            (
                "MyPool/MySecondDataset".to_string(),
                "another-password".to_string(),
            ),
            (
                "MyPool/MyThirdDataset".to_string(),
                "third-password".to_string(),
            ),
        ],
    )
}

fn main() {
    console_error_panic_hook::set_once();

    let api_impl = make_mock();

    mount_to_body(|| view! { <App api=api_impl /> })
}

#[derive(thiserror::Error, Clone, Debug)]
pub enum FrontendError {
    #[error("Web API returned an error response: {0}")]
    WebAPIResponseError(String),
}

async fn initial_query<A: ZfsRemoteHighLevel + 'static>(
    api: A,
) -> Result<(A, DatasetsFullMountState, bool), A::Error> {
    let result = join!(api.encrypted_unmounted_datasets(), api.is_permissive());

    match (result.0, result.1) {
        (Ok(r1), Ok(r2)) => Ok((api, r1, r2)),
        (Ok(_), Err(e)) => Err(e),
        (Err(e), Ok(_)) => Err(e),
        (Err(e1), Err(_e2)) => Err(e1),
    }
}

#[component]
fn App<A: ZfsRemoteHighLevel + 'static>(api: A) -> impl IntoView {
    let zfs_rows = create_local_resource(
        || (),
        move |_| {
            let api = api.clone();
            async move { initial_query(api).await }
        },
    );

    let fallback = move |errors: RwSignal<Errors>| {
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
    };

    let zfs_table_view = move || {
        zfs_rows.and_then(|(api, rows, permissive)| {
            view! { <ZfsUnlocksTable api=api.clone() unmounted_datasets=rows is_permissive=*permissive /> }
        })
    };

    view! {
        <ErrorBoundary fallback>
            <Transition fallback=move || {
                view! { <div>"Loading ZFS datasets..."</div> }
            }>
                <div>{zfs_table_view}</div>
            </Transition>
        </ErrorBoundary>
    }
}

#[allow(clippy::needless_lifetimes)]
#[component]
fn ZfsKeyPasswordInput<'a, A: ZfsRemoteHighLevel + 'static>(
    api: A,
    current_mount_state: &'a DatasetFullMountState,
) -> impl IntoView {
    let dataset_name = Arc::new(current_mount_state.dataset_name.to_string());
    let dataset_name_for_pw = dataset_name.clone();

    let api_for_pw: A = api.clone();

    let (password_in_input, set_password_in_input) = create_signal("".to_string());
    let reloaded_dataset = create_local_resource(
        move || (),
        move |_| {
            let api = api.clone();
            let dataset_name = dataset_name.clone();
            async move { api.dataset_state(&dataset_name).map(Some).await }
        },
    );

    // if there's a single argument, just use that
    let load_key_password = create_action(move |password: &String| {
        let password = password.clone();
        let mut api_for_pw: A = api_for_pw.clone();
        let dataset_name = dataset_name_for_pw.clone();
        async move { api_for_pw.load_key(&dataset_name, &password).await }
    });

    let password_field_or_key_already_loaded = move |key_loaded_result: Result<
        bool,
        <A as ZfsRemoteAPI>::Error,
    >| {
        match key_loaded_result {
            Ok(key_loaded) => view! {
                <Show when=move || !key_loaded fallback=|| view! { "Key already loaded" }>
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
                                load_key_password.dispatch(password_in_input.get());
                                reloaded_dataset.set(None);
                                reloaded_dataset.refetch();
                            }>"Submit"</button>
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
        let reloaded_dataset = reloaded_dataset.get();
        let reloaded_dataset = reloaded_dataset.flatten();
        let ds_info = reloaded_dataset.map(|ds| ds.clone().map(|m| m.key_loaded));
        match ds_info {
            Some(key_loaded) => password_field_or_key_already_loaded(key_loaded).into_view(),
            None => view! { <p>"Loading..."</p> }.into_view(),
        }
    }
}

#[allow(clippy::needless_lifetimes)]
#[component]
fn ZfsDatasetRow<'a, A: ZfsRemoteHighLevel + 'static>(
    api: A,
    current_mount_state: &'a DatasetFullMountState,
) -> impl IntoView {
    view! {
        <tr>
            <th>
                <p>{&current_mount_state.dataset_name}</p>
            </th>
            <th>
                <ZfsKeyPasswordInput api current_mount_state />
            </th>
            <th>
                <p>"<Mount button>"</p>
            </th>
        </tr>
    }
}

#[allow(clippy::needless_lifetimes)]
#[component]
fn ZfsUnlocksTable<'a, A: ZfsRemoteHighLevel + 'static>(
    api: A,
    unmounted_datasets: &'a DatasetsFullMountState,
    is_permissive: bool,
) -> impl IntoView {
    let _is_permissive = is_permissive;

    let locked_count = unmounted_datasets.states.len();

    let unmounted_datasets = (*unmounted_datasets).clone();

    view! {
        <table>
            <Show when=move || (locked_count > 0) fallback=|| view! { <NothingToUnlock /> }>
                {unmounted_datasets
                    .states
                    .values()
                    .map(|mount_data| {
                        view! { <ZfsDatasetRow api=api.clone() current_mount_state=mount_data /> }
                    })
                    .collect_view()}
            </Show>
        </table>
    }
}

#[component]
fn NothingToUnlock() -> impl IntoView {
    view! { <p>"All datasets unlocked and mounted"</p> }
}
