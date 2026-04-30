use graph_core::GraphQuery;

use crate::traits::InputSource;

pub struct TextQuerySource {
    pending: Vec<GraphQuery>,
}

impl TextQuerySource {
    pub fn new(initial_query: Option<String>) -> Self {
        let pending = initial_query
            .map(|q| vec![GraphQuery { query: q }])
            .unwrap_or_default();
        Self { pending }
    }

    pub fn submit(&mut self, query: String) {
        self.pending.push(GraphQuery { query });
    }
}

impl InputSource for TextQuerySource {
    fn poll_query(&mut self) -> Option<GraphQuery> {
        self.pending.pop()
    }
}
