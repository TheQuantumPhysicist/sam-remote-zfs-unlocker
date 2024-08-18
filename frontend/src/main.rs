use common::{
    api::{mock::ApiMock, traits::ZfsRemoteHighLevel},
    types::DatasetsMountState,
};
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

#[component]
fn App<A: ZfsRemoteHighLevel + 'static>(api: A) -> impl IntoView {
    // let is_permissive = api.is_permissive();

    let zfs_rows = create_local_resource(
        || (),
        move |_| {
            let api = api.clone();
            async move { api.unmounted_datasets().await }
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
        zfs_rows.and_then(|rows| {
            view! { <ZfsUnlocksTable unmounted_datasets=rows /> }
        })
    };

    view! {
        <ErrorBoundary fallback>
            <Transition fallback=move || {
                view! { <div>"Loading (Suspense Fallback)..."</div> }
            }>{zfs_table_view} <div></div></Transition>

        </ErrorBoundary>
    }
}

#[component]
fn ZfsPasswordInput(dataset_name: String) -> impl IntoView {
    let (password_in_input, set_name) = create_signal("Controlled".to_string());
    let (password, set_password) = create_signal("".to_string());

    // TODO: use <Show when...> to show the `component` only when the input is needed (i.e., the dataset is locked)
    // See: https://book.leptos.dev/view/06_control_flow.html
    view! {
        <tr>
            <th>
                <p>"Dataset: " {dataset_name}</p>
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
                <p>"Password is: " {password}</p>
            </th>
        </tr>
    }
}

#[component]
fn ZfsUnlocksTable<'a>(unmounted_datasets: &'a DatasetsMountState) -> impl IntoView {
    let locked_count = unmounted_datasets.datasets_mounted.len();

    view! {
        <table>
            <Show when=move || (locked_count > 0) fallback=|| view! { <NothingToUnlock /> }>
                <ZfsPasswordInput dataset_name="MyPool/MyDataset".to_string() />
            </Show>
        </table>
    }
}

#[component]
fn NothingToUnlock() -> impl IntoView {
    view! { <p>"All datasets unlocked and mounted"</p> }
}
