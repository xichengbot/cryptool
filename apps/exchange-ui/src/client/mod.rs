use std::{future::Future, pin::Pin, rc::Rc};

use exchange_types::{QueryError, TradeQuery, TradeWindow};

#[cfg(feature = "web")]
mod http;
#[cfg(feature = "desktop")]
mod native;

pub type QueryFuture = Pin<Box<dyn Future<Output = Result<TradeWindow, QueryError>> + 'static>>;

pub trait QueryClient {
    fn query_trades(&self, query: TradeQuery) -> QueryFuture;
}

#[derive(Clone)]
pub struct QueryClientHandle(Rc<dyn QueryClient>);

impl QueryClientHandle {
    pub fn query_trades(&self, query: TradeQuery) -> QueryFuture {
        self.0.query_trades(query)
    }
}

impl PartialEq for QueryClientHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, PartialEq)]
pub struct ClientBootstrap {
    pub client: Option<QueryClientHandle>,
    pub status: String,
}

#[cfg(feature = "web")]
pub fn bootstrap() -> ClientBootstrap {
    ClientBootstrap {
        client: Some(QueryClientHandle(Rc::new(http::HttpQueryClient))),
        status: "Web API 已就绪".into(),
    }
}

#[cfg(feature = "desktop")]
pub fn bootstrap() -> ClientBootstrap {
    native::bootstrap()
}
