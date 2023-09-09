#![feature(ip)]
#![feature(impl_trait_in_assoc_type)]

use hyper::client::connect::dns::Name;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::Service;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::error::ResolveError;
use trust_dns_resolver::TokioAsyncResolver;

#[derive(Clone)]
pub struct GlobalResolver {
    resolver: Arc<TokioAsyncResolver>,
}

impl GlobalResolver {
    pub fn new() -> Self {
        Self::new_with_opts(ResolverConfig::default(), ResolverOpts::default())
    }

    pub fn new_with_opts(resolver_config: ResolverConfig, resolver_opts: ResolverOpts) -> Self {
        let resolver = TokioAsyncResolver::tokio(resolver_config, resolver_opts)
            .expect("Failed to create resolver with default settings");

        GlobalResolver {
            resolver: Arc::new(resolver),
        }
    }
}

impl Default for GlobalResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Service<Name> for GlobalResolver {
    type Response = impl Iterator<Item = SocketAddr>;
    type Error = ResolveError;
    type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Name) -> Self::Future {
        let resolver = self.resolver.clone();

        Box::pin(async move {
            let res = resolver.lookup_ip(req.as_str()).await?;

            let iterator = res
                .into_iter()
                .filter(IpAddr::is_global)
                .map(|ip| SocketAddr::new(ip, 0))
                .collect::<Vec<SocketAddr>>()
                .into_iter();

            Ok(iterator)
        })
    }
}
