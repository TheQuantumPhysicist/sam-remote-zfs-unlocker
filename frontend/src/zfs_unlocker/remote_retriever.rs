use common::{
    api::traits::{ZfsRemoteAPI, ZfsRemoteHighLevel},
    types::DatasetFullMountState,
};
use futures::FutureExt;
use leptos::{create_local_resource, Resource, SignalGet, SignalSet};

#[must_use]
#[derive(Debug, Clone)]
pub struct DatasetStateResource<A: ZfsRemoteHighLevel> {
    dataset_name: String,
    res: Resource<(), Option<Result<DatasetFullMountState, <A as ZfsRemoteAPI>::Error>>>,
}

impl<A: ZfsRemoteHighLevel + 'static> DatasetStateResource<A> {
    fn make_resource(
        api: A,
        dataset_name: impl Into<String>,
        log_func: &'static impl Fn(&str),
    ) -> Resource<(), Option<Result<DatasetFullMountState, <A as ZfsRemoteAPI>::Error>>> {
        let dataset_name = dataset_name.into();
        create_local_resource(
            move || (),
            move |_| {
                let api = api.clone();
                let dataset_name = dataset_name.clone();
                // We wrap with Some, because None is used to trigger reloading after the user submits the password
                async move {
                    let dataset_retrieval_result =
                        api.encrypted_dataset_state(&dataset_name).map(Some).await;
                    if let Err(op_err) = dataset_retrieval_result.clone().transpose() {
                        log_func(&format!(
                            "Request to retrieve datasets returned an error: {op_err}"
                        ))
                    }
                    dataset_retrieval_result
                }
            },
        )
    }

    pub fn new(dataset_name: String, api: A, log_func: &'static impl Fn(&str)) -> Self {
        Self {
            res: Self::make_resource(api, dataset_name.clone(), log_func),
            dataset_name,
        }
    }

    pub fn dataset_name(&self) -> &str {
        &self.dataset_name
    }

    pub fn refresh_dataset_state(&self) {
        self.res.set(None);
        self.res.refetch();
    }

    pub fn get(&self) -> Option<Result<DatasetFullMountState, <A as ZfsRemoteAPI>::Error>> {
        // We flatten because we have 2 Option wraps:
        // 1. The Option from create_local_resource finishing
        // 2. The Option that we manually added, so that we set it to None when the user clicks on "Submit"
        self.res.get().flatten()
    }
}
