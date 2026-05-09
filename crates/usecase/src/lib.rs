use tefas_domain::QueryOperationName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryBatchRequest {
    pub operations: Vec<QueryOperationName>,
    pub requested_concurrency: Option<usize>,
}

impl QueryBatchRequest {
    pub fn new(operations: Vec<QueryOperationName>, requested_concurrency: Option<usize>) -> Self {
        Self {
            operations,
            requested_concurrency,
        }
    }

    pub fn effective_concurrency(&self) -> usize {
        if let Some(value) = self.requested_concurrency {
            return value.max(1);
        }

        // Greenfield default policy:
        // - Small query sets: conservative parallelism (2)
        // - Wide query sets: stronger fan-out (4)
        if self.operations.len() >= 8 { 4 } else { 2 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryBatchPlan {
    pub operations: Vec<QueryOperationName>,
    pub concurrency: usize,
}

pub fn build_query_batch_plan(request: QueryBatchRequest) -> QueryBatchPlan {
    QueryBatchPlan {
        concurrency: request.effective_concurrency(),
        operations: request.operations,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FundpageBatchRequest {
    pub codes: Vec<String>,
    pub requested_concurrency: Option<usize>,
}

impl FundpageBatchRequest {
    pub fn new(codes: Vec<String>, requested_concurrency: Option<usize>) -> Self {
        Self {
            codes,
            requested_concurrency,
        }
    }

    pub fn effective_concurrency(&self) -> usize {
        self.requested_concurrency.unwrap_or(2).max(1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FundpageBatchPlan {
    pub codes: Vec<String>,
    pub concurrency: usize,
}

pub fn build_fundpage_batch_plan(request: FundpageBatchRequest) -> FundpageBatchPlan {
    let concurrency = request.effective_concurrency();
    FundpageBatchPlan {
        codes: request.codes,
        concurrency,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchBatchRequest {
    pub urls: Vec<String>,
    pub requested_concurrency: Option<usize>,
    pub backend_default_concurrency: usize,
}

impl FetchBatchRequest {
    pub fn new(
        urls: Vec<String>,
        requested_concurrency: Option<usize>,
        backend_default_concurrency: usize,
    ) -> Self {
        Self {
            urls,
            requested_concurrency,
            backend_default_concurrency,
        }
    }

    pub fn effective_concurrency(&self) -> usize {
        self.requested_concurrency
            .unwrap_or(self.backend_default_concurrency)
            .max(1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchBatchPlan {
    pub urls: Vec<String>,
    pub concurrency: usize,
}

pub fn build_fetch_batch_plan(request: FetchBatchRequest) -> FetchBatchPlan {
    let concurrency = request.effective_concurrency();
    FetchBatchPlan {
        urls: request.urls,
        concurrency,
    }
}

#[cfg(test)]
mod tests;
