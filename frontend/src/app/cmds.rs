use common::{
    api::{
        mock::ApiMock,
        routed::ApiRouteImpl,
        traits::{ZfsRemoteAPI, ZfsRemoteHighLevel},
    },
    types::{AvailableCustomCommands, CustomCommandInfo, RunCommandOutput},
};
use leptos::{
    component, create_action, create_local_resource, create_signal, event_target_value, view,
    CollectView, ErrorBoundary, IntoView, Show, SignalGet, SignalSet, Transition,
};

use crate::{
    app::{config_reader::retrieve_config, error_fallback, log},
    images::RandomLoadingImage,
};

use super::command_communicator::{CommandResource, OutputExecutionResult};

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
                log(&format!("Config loaded: {:?}", config));
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

async fn custom_commands_initial_query<A: ZfsRemoteHighLevel + 'static>(
    api: A,
) -> Result<(A, AvailableCustomCommands), A::Error> {
    let result = api.list_available_commands().await;

    result.map(|r| (api, r))
}

#[component]
pub fn CommandsTable<A: ZfsRemoteHighLevel + 'static>(api: A) -> impl IntoView {
    log("Creating custom commands table");

    let zfs_rows = create_local_resource(
        || (),
        move |_| {
            let api = api.clone();
            async move { custom_commands_initial_query(api).await }
        },
    );

    let commands_table_view = move || {
        zfs_rows.and_then(|(api, rows)| {
            view! { <CommandCallsTable api=api.clone() available_commands=rows /> }
        })
    };

    view! {
        <ErrorBoundary fallback=error_fallback>
            <Transition fallback=move || {
                view! {
                    <div class="custom-commands-loading-page">
                        <RandomLoadingImage />
                    </div>
                }
            }>
                <div>{commands_table_view}</div>
            </Transition>
        </ErrorBoundary>
    }
}

#[allow(clippy::needless_lifetimes)]
#[component]
fn CommandCallsTable<'a, A: ZfsRemoteHighLevel + 'static>(
    api: A,
    available_commands: &'a AvailableCustomCommands,
) -> impl IntoView {
    log(&format!("Commands found: {:?}", available_commands));
    let rows_count = available_commands.commands.len();

    let available_commands = (*available_commands).clone();

    view! {
        <div class="custom-commands-table-container">
            <Show when=move || (rows_count > 0) fallback=|| view! { <NoCommandsAvailable /> }>
                <table class="custom-commands-table">
                    <thead>
                        <CommandRow api=api.clone() command_info=None />
                    </thead>
                    <tbody>
                        {available_commands
                            .commands
                            .iter()
                            .map(|command_info| {
                                view! {
                                    <CommandRow api=api.clone() command_info=Some(command_info) />
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
fn NoCommandsAvailable() -> impl IntoView {
    view! { <p>"No commands available to show"</p> }
}

enum CustomCommandsTableColumnDefinition {
    Name,
    InputAndSubmit,
    ErrorCode,
    Stdout,
    Stderr,
}

#[component]
fn CommandTableCell<A: ZfsRemoteHighLevel + 'static>(
    command_resource: Option<CommandResource<A>>,
    column: CustomCommandsTableColumnDefinition,
) -> impl IntoView {
    match column {
        CustomCommandsTableColumnDefinition::Name => match command_resource {
            Some(res) => view! {
                <div class="table-cell-cmd-label">
                    <p>{&res.command_info().label}</p>
                </div>
            }
            .into_view(),
            None => view! { <p>"Command label"</p> }.into_view(),
        },
        CustomCommandsTableColumnDefinition::InputAndSubmit => match command_resource {
            Some(res) => view! { <CommandExecuteCell command_resource=res /> }.into_view(),
            None => view! { <p>"Execute"</p> }.into_view(),
        },
        CustomCommandsTableColumnDefinition::ErrorCode => match command_resource {
            Some(ds) => view! { <ErrorCodeCell command_resource=ds /> }.into_view(),
            None => view! { <p>"Error code"</p> }.into_view(),
        },
        CustomCommandsTableColumnDefinition::Stdout => match command_resource {
            Some(ds) => view! { <StringOutputCell command_resource=ds extractor=|o| o.stdout.to_string() /> }
                .into_view(),
            None => view! { <p>"Stdout output"</p> }.into_view(),
        },
        CustomCommandsTableColumnDefinition::Stderr => match command_resource {
            Some(ds) => view! { <StringOutputCell command_resource=ds extractor=|o| o.stderr.to_string() /> }
                .into_view(),
            None => view! { <p>"Stderr output"</p> }.into_view(),
        },
    }
}

#[component]
fn CommandExecuteCell<A: ZfsRemoteHighLevel + 'static>(
    command_resource: CommandResource<A>,
) -> impl IntoView {
    let (stdin_in_input, set_stdin_in_input) = create_signal("".to_string());

    let command_resource_for_action = command_resource.clone();

    // This action takes the action from the user, the click, and sends it to the API to execute the command
    let call_command = create_action(move |stdin_string: &String| {
        let command_resource = command_resource_for_action.clone();
        let stdin_string = command_resource
            .command_info()
            .allow_stdin
            .then_some(stdin_string.clone());
        async move {
            // We reset first, to trigger the loading animation
            command_resource.set_command_state_as_loading();
            command_resource.call_command(stdin_string)
        }
    });

    // This contains the text field + submit button objects, depending on whether stdin is allowed or not
    move || {
        let stdin_field = if command_resource.command_info().allow_stdin {
            view! {
                <input
                    type="password"
                    on:input=move |ev| {
                        set_stdin_in_input.set(event_target_value(&ev));
                    }
                    prop:value=stdin_in_input
                />
                <br />
            }
            .into_view()
        } else {
            view! {}.into_view()
        };
        view! {
            {stdin_field}
            <button on:click=move |_| {
                call_command.dispatch(stdin_in_input.get());
            }>"Execute command"</button>
        }
    }
}

#[component]
fn StdOutputFormatted(output: String) -> impl IntoView {
    view! { <p>{output}</p> }
}

#[component]
fn StringOutputCell<A: ZfsRemoteHighLevel + 'static>(
    command_resource: CommandResource<A>,
    extractor: impl Fn(&RunCommandOutput) -> String + 'static,
) -> impl IntoView {
    // This contains the text field + submit button objects, depending on whether stdin is allowed
    let finished_view =
        move |output_result: Result<RunCommandOutput, <A as ZfsRemoteAPI>::Error>| {
            match output_result {
                Ok(output) => view! { <StdOutputFormatted output=extractor(&output) /> },
                Err(e) => view! {
                    "Retrieval of error code failed: "
                    {e.to_string()}
                }
                .into_view(),
            }
        };

    move || {
        let command_result = command_resource.get();
        match command_result {
            OutputExecutionResult::InitialState => view! {}.into_view(),
            OutputExecutionResult::Loading => view! { <RandomLoadingImage /> }.into_view(),
            OutputExecutionResult::RanAtLeastOnce(output) => finished_view(output).into_view(),
        }
    }
}

#[component]
fn ErrorCodeFromOutput(output: RunCommandOutput) -> impl IntoView {
    view! { <p>{output.error_code}</p> }
}

#[component]
fn ErrorCodeCell<A: ZfsRemoteHighLevel + 'static>(
    command_resource: CommandResource<A>,
) -> impl IntoView {
    // This contains the text field + submit button objects, depending on whether stdin is allowed
    let finished_view =
        move |output_result: Result<RunCommandOutput, <A as ZfsRemoteAPI>::Error>| {
            match output_result {
                Ok(output) => view! { <ErrorCodeFromOutput output /> },
                Err(e) => view! {
                    "Retrieval of error code failed: "
                    {e.to_string()}
                }
                .into_view(),
            }
        };

    move || {
        let command_result = command_resource.get();
        match command_result {
            OutputExecutionResult::InitialState => view! {}.into_view(),
            OutputExecutionResult::Loading => view! { <RandomLoadingImage /> }.into_view(),
            OutputExecutionResult::RanAtLeastOnce(output) => finished_view(output).into_view(),
        }
    }
}

#[allow(clippy::needless_lifetimes)]
#[component]
fn CommandRow<'a, A: ZfsRemoteHighLevel + 'static>(
    api: A,
    command_info: Option<&'a CustomCommandInfo>,
) -> impl IntoView {
    let command_resource = command_info.map(|m| CommandResource::new(m.clone(), api, &log));

    view! {
        <tr>
            <th>
                <CommandTableCell
                    command_resource=command_resource.clone()
                    column=CustomCommandsTableColumnDefinition::Name
                />
            </th>
            <th>
                <CommandTableCell
                    command_resource=command_resource.clone()
                    column=CustomCommandsTableColumnDefinition::InputAndSubmit
                />
            </th>
            <th>
                <CommandTableCell
                    command_resource=command_resource.clone()
                    column=CustomCommandsTableColumnDefinition::ErrorCode
                />
            </th>
            <th>
                <CommandTableCell
                    command_resource=command_resource.clone()
                    column=CustomCommandsTableColumnDefinition::Stdout
                />
            </th>
            <th>
                <CommandTableCell
                    command_resource=command_resource.clone()
                    column=CustomCommandsTableColumnDefinition::Stderr
                />
            </th>
        </tr>
    }
}
