use common::{
    api::{
        mock::ApiMock,
        routed::ApiRouteImpl,
        traits::{ZfsRemoteAPI, ZfsRemoteHighLevel},
    },
    types::{AvailableCustomCommands, CustomCommandInfo, RunCommandOutput},
};
use leptos::{
    component, create_action, create_local_resource, create_rw_signal, create_signal,
    event_target_value, view, CollectView, ErrorBoundary, IntoView, Show, SignalGet, SignalSet,
    Transition,
};
use leptos_icons::Icon;

use crate::{
    app::{config_reader::retrieve_config, error_fallback, log, modal::Modal},
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
    view! { <p>"No commands available to execute"</p> }
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
            Some(ds) => view! {
                <StringOutputCell
                    command_resource=ds
                    extractor=|o| o.stdout.to_string()
                    output_name="stdout".to_string()
                />
            }
            .into_view(),
            None => view! { <p>"Stdout output"</p> }.into_view(),
        },
        CustomCommandsTableColumnDefinition::Stderr => match command_resource {
            Some(ds) => view! {
                <StringOutputCell
                    command_resource=ds
                    extractor=|o| o.stderr.to_string()
                    output_name="stderr".to_string()
                />
            }
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
            .stdin_allow
            .then_some(stdin_string.clone());
        async move {
            // We reset first, to trigger the loading animation
            command_resource.set_command_state_as_loading();
            command_resource.call_command(stdin_string)
        }
    });

    // This contains the text field + submit button objects, depending on whether stdin is allowed or not
    move || {
        let stdin_field = if command_resource.command_info().stdin_allow {
            view! {
                <input
                    type=view! {
                        {if command_resource.command_info().stdin_is_password {
                            "password"
                        } else {
                            "text"
                        }}
                    }
                    placeholder=command_resource.command_info().stdin_text_placeholder.clone()
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
fn StdOutputFormatted(output: String, button_label: String) -> impl IntoView {
    let open_dialog = create_rw_signal(false);

    if !output.trim().is_empty() {
        view! {
            <button on:click=move |_| open_dialog.set(true)>{button_label.clone()}</button>
            <Modal
                open=open_dialog
                on_close=move || {}
                children=move || {
                    {
                        view! { <p class="custom-commands-std-output">{output}</p> }
                    }
                        .into_view()
                        .into()
                }
            />
        }
        .into_view()
    } else {
        view! { <NothingToShowIcon /> }.into_view()
    }
}

#[component]
fn StringOutputCell<A: ZfsRemoteHighLevel + 'static>(
    command_resource: CommandResource<A>,
    extractor: impl Fn(&RunCommandOutput) -> String + 'static,
    output_name: String,
) -> impl IntoView {
    // This contains the text field + submit button objects, depending on whether stdin is allowed
    let finished_view =
        move |output_result: Result<RunCommandOutput, <A as ZfsRemoteAPI>::Error>| {
            match output_result {
                Ok(output) => {
                    view! {
                        <StdOutputFormatted
                            output=extractor(&output)
                            button_label=format!("Show {output_name}")
                        />
                    }
                }
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
            OutputExecutionResult::InitialState => view! { <NothingToShowIcon /> }.into_view(),
            OutputExecutionResult::Loading => view! { <RandomLoadingImage /> }.into_view(),
            OutputExecutionResult::RanAtLeastOnce(output) => finished_view(output).into_view(),
        }
    }
}

#[component]
fn ErrorCodeFromOutput(output: RunCommandOutput) -> impl IntoView {
    if output.error_code == 0 {
        view! { <CheckFor0ErrorCode /> }.into_view()
    } else {
        view! { <p style="color: red;">{output.error_code}</p> }.into_view()
    }
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
            OutputExecutionResult::InitialState => view! { <NothingToShowIcon /> }.into_view(),
            OutputExecutionResult::Loading => view! { <RandomLoadingImage /> }.into_view(),
            OutputExecutionResult::RanAtLeastOnce(output) => finished_view(output).into_view(),
        }
    }
}

#[component]
fn CheckFor0ErrorCode() -> impl IntoView {
    view! {
        <div style="font-size: 2em; color: #8f39d3;" title="Success - exit code 0">
            <Icon icon=icondata::VsCheck style="color: green" />
        </div>
    }
}

#[component]
fn NothingToShowIcon() -> impl IntoView {
    view! {
        <div style="font-size: 1em; color: #8f39d3;" title="Nothing to show">
            <Icon icon=icondata::LuCircleSlash2 style="color: gray" />
        </div>
    }
}

#[component]
fn CheckMarkForSuccessIcon() -> impl IntoView {
    view! {
        <div style="font-size: 1em; color: #8f39d3;">
            <Icon icon=icondata::VsCheck style="color: gray" />
        </div>
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
