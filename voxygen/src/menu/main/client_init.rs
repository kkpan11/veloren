use client::{
    Client, ClientInitStage, ServerInfo,
    addr::ConnectionArgs,
    error::{Error as ClientError, NetworkConnectError, NetworkError},
};
use common_net::msg::ClientType;
use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::runtime;
use tracing::{trace, warn};

#[derive(Debug)]
#[expect(clippy::enum_variant_names)] //TODO: evaluate ClientError ends with Enum name
pub enum Error {
    ClientError {
        error: ClientError,
        mismatched_server_info: Option<ServerInfo>,
    },
    ClientCrashed,
    ServerNotFound,
}

pub enum Msg {
    IsAuthTrusted(String),
    Done(Result<Client, Error>),
}

pub struct AuthTrust(String, bool);

// Used to asynchronously parse the server address, resolve host names,
// and create the client (which involves establishing a connection to the
// server).
pub struct ClientInit {
    rx: Receiver<Msg>,
    stage_rx: Receiver<ClientInitStage>,
    trust_tx: Sender<AuthTrust>,
    cancel: Arc<AtomicBool>,
}
impl ClientInit {
    pub fn new(
        connection_args: ConnectionArgs,
        username: String,
        password: String,
        runtime: Arc<runtime::Runtime>,
        locale: Option<String>,
        config_dir: &Path,
        client_type: ClientType,
    ) -> Self {
        let (tx, rx) = unbounded();
        let (trust_tx, trust_rx) = unbounded();
        let (init_stage_tx, init_stage_rx) = unbounded();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel2 = Arc::clone(&cancel);

        let runtime2 = Arc::clone(&runtime);
        let config_dir = config_dir.to_path_buf();

        runtime.spawn(async move {
            let trust_fn = |auth_server: &str| {
                let _ = tx.send(Msg::IsAuthTrusted(auth_server.to_string()));
                trust_rx
                    .recv()
                    .map(|AuthTrust(server, trust)| trust && server == *auth_server)
                    .unwrap_or(false)
            };

            let mut last_err = None;

            const FOUR_MINUTES_RETRIES: u64 = 48;
            'tries: for _ in 0..FOUR_MINUTES_RETRIES {
                if cancel2.load(Ordering::Relaxed) {
                    break;
                }
                let mut mismatched_server_info = None;
                match Client::new(
                    connection_args.clone(),
                    Arc::clone(&runtime2),
                    &mut mismatched_server_info,
                    &username,
                    &password,
                    locale.clone(),
                    trust_fn,
                    &|stage| {
                        let _ = init_stage_tx.send(stage);
                    },
                    crate::ecs::sys::add_local_systems,
                    config_dir.clone(),
                    client_type,
                )
                .await
                {
                    Ok(client) => {
                        let _ = tx.send(Msg::Done(Ok(client)));
                        tokio::task::block_in_place(move || drop(runtime2));
                        return;
                    },
                    Err(ClientError::NetworkErr(NetworkError::ConnectFailed(
                        NetworkConnectError::Io(e),
                    ))) => {
                        warn!(?e, "Failed to connect to the server. Retrying...");
                    },
                    Err(e) => {
                        trace!(?e, "Aborting server connection attempt");
                        last_err = Some(Error::ClientError {
                            error: e,
                            mismatched_server_info,
                        });
                        break 'tries;
                    },
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }

            // Parsing/host name resolution successful but no connection succeeded
            // If last_err is None this typically means there was no server up at the input
            // address and all the attempts timed out.
            let _ = tx.send(Msg::Done(Err(last_err.unwrap_or(Error::ServerNotFound))));

            // Safe drop runtime
            tokio::task::block_in_place(move || drop(runtime2));
        });

        ClientInit {
            rx,
            stage_rx: init_stage_rx,
            trust_tx,
            cancel,
        }
    }

    /// Poll if the thread is complete.
    /// Returns None if the thread is still running, otherwise returns the
    /// Result of client creation.
    pub fn poll(&self) -> Option<Msg> {
        match self.rx.try_recv() {
            Ok(msg) => Some(msg),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Msg::Done(Err(Error::ClientCrashed))),
        }
    }

    /// Poll for connection stage updates from the client
    pub fn stage_update(&self) -> Option<ClientInitStage> { self.stage_rx.try_recv().ok() }

    /// Report trust status of auth server
    pub fn auth_trust(&self, auth_server: String, trusted: bool) {
        let _ = self.trust_tx.send(AuthTrust(auth_server, trusted));
    }

    pub fn cancel(&mut self) { self.cancel.store(true, Ordering::Relaxed); }
}

impl Drop for ClientInit {
    fn drop(&mut self) { self.cancel(); }
}
