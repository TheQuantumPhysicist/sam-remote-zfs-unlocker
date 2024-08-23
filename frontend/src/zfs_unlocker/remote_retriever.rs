use common::{
    api::traits::{ZfsRemoteAPI, ZfsRemoteHighLevel},
    types::DatasetFullMountState,
};
use leptos::{Resource, SignalGet, SignalSet};

#[must_use]
#[derive(Debug, Clone)]
pub struct DatasetStateResource<A: ZfsRemoteHighLevel> {
    dataset_name: String,
    res: Resource<(), Option<Result<DatasetFullMountState, <A as ZfsRemoteAPI>::Error>>>,
}

impl<A: ZfsRemoteHighLevel> DatasetStateResource<A> {
    pub fn new(
        dataset_name: String,
        resource: Resource<(), Option<Result<DatasetFullMountState, <A as ZfsRemoteAPI>::Error>>>,
    ) -> Self {
        Self {
            dataset_name,
            res: resource,
        }
    }

    pub fn dataset_name(&self) -> &str {
        &self.dataset_name
    }

    pub fn refresh(&self) {
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
