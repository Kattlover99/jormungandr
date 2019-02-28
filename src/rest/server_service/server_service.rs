use actix_net::server::Server;
use actix_web::actix::{Addr, MailboxError, System};
use actix_web::server;
use actix_web::server::{IntoHttpHandler, StopServer};
use futures::Future;
use native_tls::{Identity, TlsAcceptor};
use rest::server_service::{Error, ServerResult, ServerServiceBuilder};
use std::fs;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::sync::mpsc::sync_channel;
use std::thread;

#[derive(Clone)]
pub struct ServerService {
    addr: Addr<Server>,
}

impl ServerService {
    pub fn builder(
        pkcs12: Option<PathBuf>,
        address: SocketAddr,
        prefix: impl Into<String>,
    ) -> ServerServiceBuilder {
        ServerServiceBuilder::new(pkcs12, address, prefix)
    }

    pub fn start<F, H>(
        pkcs12: Option<PathBuf>,
        address: SocketAddr,
        handler: F,
    ) -> ServerResult<Self>
    where
        F: Fn() -> H + Send + Clone + 'static,
        H: IntoHttpHandler + 'static,
    {
        let tls = load_tls_acceptor(pkcs12)?;
        let (sender, receiver) = sync_channel::<ServerResult<ServerService>>(0);
        thread::spawn(move || {
            let actix_system = System::builder().build();
            let server_handler = start_server_curr_actix_system(address, tls, handler);
            let run_system = server_handler.is_ok();
            let _ = sender.send(server_handler);
            if run_system {
                actix_system.run();
            }
        });
        receiver.recv().unwrap()
    }

    pub fn stop(&self) -> impl Future<Item = (), Error = Error> {
        self.addr
            .send(StopServer { graceful: true })
            .then(|result| match result {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(_)) => Err(Error::ServerStopFailed),
                Err(MailboxError::Closed) => Err(Error::ServerAlreadyStopped),
                Err(MailboxError::Timeout) => Err(Error::ServerStopTimeout),
            })
    }
}

fn load_tls_acceptor(pkcs12_opt: Option<PathBuf>) -> ServerResult<Option<TlsAcceptor>> {
    let pkcs12_path = match pkcs12_opt {
        Some(pkcs12) => pkcs12,
        None => return Ok(None),
    };
    let pkcs12_data = fs::read(pkcs12_path).map_err(|e| Error::Pkcs12LoadFailed(e))?;
    let identity = Identity::from_pkcs12(&pkcs12_data, "").map_err(|e| Error::Pkcs12Invalid(e))?;
    let tls = TlsAcceptor::new(identity).map_err(|e| Error::Pkcs12Invalid(e))?;
    Ok(Some(tls))
}

fn start_server_curr_actix_system<F, H>(
    address: impl ToSocketAddrs,
    tls_opt: Option<TlsAcceptor>,
    handler: F,
) -> ServerResult<ServerService>
where
    F: Fn() -> H + Send + Clone + 'static,
    H: IntoHttpHandler + 'static,
{
    let server = server::new(handler).system_exit().disable_signals();
    let binded_server = match tls_opt {
        Some(tls) => server.bind_tls(address, tls),
        None => server.bind(address),
    }
    .map_err(|err| Error::BindFailed(err))?;
    Ok(ServerService {
        addr: binded_server.start(),
    })
}
