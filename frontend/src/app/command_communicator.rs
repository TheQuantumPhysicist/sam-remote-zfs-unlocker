use common::{
    api::traits::{ZfsRemoteAPI, ZfsRemoteHighLevel},
    types::{CustomCommandPublicInfo, RunCommandOutput},
};
use leptos::{
    create_local_resource, create_signal, ReadSignal, Resource, SignalGet, SignalGetUntracked,
    SignalSet, WriteSignal,
};

#[derive(Debug, Clone)]
pub enum OutputExecutionResult<T> {
    InitialState,
    Loading,
    RanAtLeastOnce(T),
}

#[must_use]
#[derive(Debug, Clone)]
pub struct CommandResource<A: ZfsRemoteHighLevel> {
    command_info: CustomCommandPublicInfo,
    res: Resource<(), OutputExecutionResult<Result<RunCommandOutput, <A as ZfsRemoteAPI>::Error>>>,
    set_stdin: WriteSignal<Option<String>>,
}

impl<A: ZfsRemoteHighLevel + 'static> CommandResource<A> {
    fn make_resource(
        api: A,
        stdin_signal: ReadSignal<Option<String>>,
        command_info: CustomCommandPublicInfo,
        log_func: &'static impl Fn(&str),
    ) -> Resource<(), OutputExecutionResult<Result<RunCommandOutput, <A as ZfsRemoteAPI>::Error>>>
    {
        let (first_run, set_first_run) = create_signal(false);

        create_local_resource(
            move || (),
            move |_| {
                let mut api = api.clone();
                let endpoint = command_info.endpoint.clone();
                async move {
                    if first_run.get_untracked() {
                        let command_run_result = api
                            .call_custom_command(&endpoint, stdin_signal.get().as_deref())
                            .await;
                        if let Err(ref op_err) = command_run_result {
                            log_func(&format!(
                                "Request to retrieve datasets returned an error: {op_err}"
                            ))
                        } else {
                            log_func(&format!("Executed command: {endpoint}"))
                        }
                        OutputExecutionResult::RanAtLeastOnce(command_run_result)
                    } else {
                        set_first_run.set(true);
                        OutputExecutionResult::InitialState
                    }
                }
            },
        )
    }

    pub fn new(
        command_info: CustomCommandPublicInfo,
        api: A,
        log_func: &'static impl Fn(&str),
    ) -> Self {
        let (stdin, set_stdin) = create_signal(None);
        Self {
            res: Self::make_resource(api, stdin, command_info.clone(), log_func),
            command_info,
            set_stdin,
        }
    }

    pub fn command_info(&self) -> &CustomCommandPublicInfo {
        &self.command_info
    }

    pub fn set_command_state_as_loading(&self) {
        self.res.set(OutputExecutionResult::Loading);
    }

    pub fn call_command(&self, stdin_string: Option<String>) {
        self.set_stdin.set(stdin_string);
        self.res.refetch();
    }

    pub fn get(
        &self,
    ) -> OutputExecutionResult<Result<RunCommandOutput, <A as ZfsRemoteAPI>::Error>> {
        self.res.get().unwrap_or(OutputExecutionResult::Loading)
    }
}
