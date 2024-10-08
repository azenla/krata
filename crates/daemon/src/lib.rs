use crate::db::ip::IpReservationStore;
use crate::db::zone::ZoneStore;
use crate::db::KrataDatabase;
use crate::ip::assignment::IpAssignment;
use anyhow::{anyhow, Result};
use config::DaemonConfig;
use console::{DaemonConsole, DaemonConsoleHandle};
use control::DaemonControlService;
use devices::DaemonDeviceManager;
use event::{DaemonEventContext, DaemonEventGenerator};
use idm::{DaemonIdm, DaemonIdmHandle};
use ipnetwork::{Ipv4Network, Ipv6Network};
use krata::{dial::ControlDialAddress, v1::control::control_service_server::ControlServiceServer};
use krataoci::{packer::service::OciPackerService, registry::OciPlatform};
use kratart::Runtime;
use log::{debug, info};
use reconcile::zone::ZoneReconciler;
use std::path::Path;
use std::time::Duration;
use std::{net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc};
use tokio::{
    fs,
    net::UnixListener,
    sync::mpsc::{channel, Sender},
    task::JoinHandle,
};
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::{Identity, Server, ServerTlsConfig};
use uuid::Uuid;
use zlt::ZoneLookupTable;

pub mod command;
pub mod config;
pub mod console;
pub mod control;
pub mod db;
pub mod devices;
pub mod event;
pub mod idm;
pub mod ip;
pub mod metrics;
pub mod oci;
pub mod reconcile;
pub mod zlt;
pub struct Daemon {
    store: String,
    _config: Arc<DaemonConfig>,
    zlt: ZoneLookupTable,
    devices: DaemonDeviceManager,
    zones: ZoneStore,
    ip: IpAssignment,
    events: DaemonEventContext,
    zone_reconciler_task: JoinHandle<()>,
    zone_reconciler_notify: Sender<Uuid>,
    generator_task: JoinHandle<()>,
    idm: DaemonIdmHandle,
    console: DaemonConsoleHandle,
    packer: OciPackerService,
    runtime: Runtime,
}

const ZONE_RECONCILER_QUEUE_LEN: usize = 1000;

impl Daemon {
    pub async fn new(store: String) -> Result<Self> {
        let store_dir = PathBuf::from(store.clone());
        debug!("loading configuration");
        let mut config_path = store_dir.clone();
        config_path.push("config.toml");

        let config = DaemonConfig::load(&config_path).await?;
        let config = Arc::new(config);
        debug!("initializing device manager");
        let devices = DaemonDeviceManager::new(config.clone());

        debug!("validating image cache directory");
        let mut image_cache_dir = store_dir.clone();
        image_cache_dir.push("cache");
        image_cache_dir.push("image");
        fs::create_dir_all(&image_cache_dir).await?;

        debug!("loading zone0 uuid");
        let mut host_uuid_path = store_dir.clone();
        host_uuid_path.push("host.uuid");
        let host_uuid = if host_uuid_path.is_file() {
            let content = fs::read_to_string(&host_uuid_path).await?;
            Uuid::from_str(content.trim()).ok()
        } else {
            None
        };

        let host_uuid = if let Some(host_uuid) = host_uuid {
            host_uuid
        } else {
            let generated = Uuid::new_v4();
            let mut string = generated.to_string();
            string.push('\n');
            fs::write(&host_uuid_path, string).await?;
            generated
        };

        debug!("validating zone asset directories");
        let initrd_path = detect_zone_path(&store, "initrd")?;
        let kernel_path = detect_zone_path(&store, "kernel")?;
        let addons_path = detect_zone_path(&store, "addons.squashfs")?;

        debug!("initializing caches and hydrating zone state");
        let seed = config.oci.seed.clone().map(PathBuf::from);
        let packer = OciPackerService::new(seed, &image_cache_dir, OciPlatform::current()).await?;
        debug!("initializing core runtime");
        let runtime = Runtime::new().await?;
        let zlt = ZoneLookupTable::new(0, host_uuid);
        let db_path = format!("{}/krata.db", store);
        let database = KrataDatabase::open(Path::new(&db_path))?;
        let zones = ZoneStore::open(database.clone())?;
        let (zone_reconciler_notify, zone_reconciler_receiver) =
            channel::<Uuid>(ZONE_RECONCILER_QUEUE_LEN);
        debug!("starting IDM service");
        let idm = DaemonIdm::new(zlt.clone()).await?;
        let idm = idm.launch().await?;
        debug!("initializing console interfaces");
        let console = DaemonConsole::new(zlt.clone()).await?;
        let console = console.launch().await?;
        let (events, generator) =
            DaemonEventGenerator::new(zones.clone(), zone_reconciler_notify.clone(), idm.clone())
                .await?;
        let runtime_for_reconciler = runtime.dupe().await?;
        let ipv4_network = Ipv4Network::from_str(&config.network.ipv4.subnet)?;
        let ipv6_network = Ipv6Network::from_str(&config.network.ipv6.subnet)?;
        let ip_reservation_store = IpReservationStore::open(database)?;
        let ip =
            IpAssignment::new(host_uuid, ipv4_network, ipv6_network, ip_reservation_store).await?;
        debug!("initializing zone reconciler");
        let zone_reconciler = ZoneReconciler::new(
            devices.clone(),
            zlt.clone(),
            zones.clone(),
            events.clone(),
            runtime_for_reconciler,
            packer.clone(),
            zone_reconciler_notify.clone(),
            kernel_path,
            initrd_path,
            addons_path,
            ip.clone(),
            config.clone(),
        )?;

        let zone_reconciler_task = zone_reconciler.launch(zone_reconciler_receiver).await?;
        let generator_task = generator.launch().await?;

        // TODO: Create a way of abstracting early init tasks in kratad.
        // TODO: Make initial power management policy configurable.
        let power = runtime.power_management_context().await?;
        power.set_smt_policy(true).await?;
        power
            .set_scheduler_policy("performance".to_string())
            .await?;
        info!("power management initialized");

        info!("krata daemon initialized");
        Ok(Self {
            store,
            _config: config,
            zlt,
            devices,
            zones,
            ip,
            events,
            zone_reconciler_task,
            zone_reconciler_notify,
            generator_task,
            idm,
            console,
            packer,
            runtime,
        })
    }

    pub async fn listen(&mut self, addr: ControlDialAddress) -> Result<()> {
        debug!("starting control service");
        let control_service = DaemonControlService::new(
            self.zlt.clone(),
            self.devices.clone(),
            self.events.clone(),
            self.console.clone(),
            self.idm.clone(),
            self.zones.clone(),
            self.ip.clone(),
            self.zone_reconciler_notify.clone(),
            self.packer.clone(),
            self.runtime.clone(),
        );

        let mut server = Server::builder();

        if let ControlDialAddress::Tls {
            host: _,
            port: _,
            insecure,
        } = &addr
        {
            let mut tls_config = ServerTlsConfig::new();
            if !insecure {
                let certificate_path = format!("{}/tls/daemon.pem", self.store);
                let key_path = format!("{}/tls/daemon.key", self.store);
                tls_config = tls_config.identity(Identity::from_pem(certificate_path, key_path));
            }
            server = server.tls_config(tls_config)?;
        }

        server = server.http2_keepalive_interval(Some(Duration::from_secs(10)));

        let server = server.add_service(ControlServiceServer::new(control_service));
        info!("listening on address {}", addr);
        match addr {
            ControlDialAddress::UnixSocket { path } => {
                let path = PathBuf::from(path);
                if path.exists() {
                    fs::remove_file(&path).await?;
                }
                let listener = UnixListener::bind(path)?;
                let stream = UnixListenerStream::new(listener);
                server.serve_with_incoming(stream).await?;
            }

            ControlDialAddress::Tcp { host, port } => {
                let address = format!("{}:{}", host, port);
                server.serve(SocketAddr::from_str(&address)?).await?;
            }

            ControlDialAddress::Tls {
                host,
                port,
                insecure: _,
            } => {
                let address = format!("{}:{}", host, port);
                server.serve(SocketAddr::from_str(&address)?).await?;
            }
        }
        Ok(())
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        self.zone_reconciler_task.abort();
        self.generator_task.abort();
    }
}

fn detect_zone_path(store: &str, name: &str) -> Result<PathBuf> {
    let mut path = PathBuf::from(format!("{}/zone/{}", store, name));
    if path.is_file() {
        return Ok(path);
    }

    path = PathBuf::from(format!("/usr/share/krata/zone/{}", name));
    if path.is_file() {
        return Ok(path);
    }
    Err(anyhow!("unable to find required zone file: {}", name))
}
