use graph_core::GraphQuery;

pub trait InputSource: Send + Sync {
    fn poll_query(&mut self) -> Option<GraphQuery>;
}
