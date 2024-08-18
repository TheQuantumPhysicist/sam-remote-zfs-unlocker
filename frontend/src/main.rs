use common::{
    api::{mock::ApiMock, traits::ZfsRemoteHighLevel},
    types::{DatasetFullMountState, DatasetsFullMountState},
};
use futures::join;
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
                view! { <div>"Loading (Suspense Fallback)..."</div> }
            }>
                <div>{zfs_table_view}</div>
            </Transition>

        </ErrorBoundary>
    }
}

#[component]
fn ZfsPasswordInput<'a, A: ZfsRemoteHighLevel + 'static>(
    api: A,
    current_mount_state: &'a DatasetFullMountState,
) -> impl IntoView {
    let _api = api;

    let (password_in_input, set_name) = create_signal("Controlled".to_string());
    let (password, set_password) = create_signal("".to_string());

    view! {
        <tr>
            <th>
                <p>{&current_mount_state.dataset_name}</p>
            </th>
            <th>
                <input
                    type="password"
                    on:input=move |ev| {
                        set_name.set(event_target_value(&ev));
                    }

                    prop:value=password_in_input
                />
                <button on:click=move |_| set_password.set(password_in_input.get()) {}>
                    "Submit"
                </button>
            </th>
            <th>
                <p>"Password is: " {password}</p>
            </th>
        </tr>
    }
}

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
                        view! {
                            <ZfsPasswordInput api=api.clone() current_mount_state=mount_data />
                        }
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
