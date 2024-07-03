use std::{
    collections::HashMap,
    sync::atomic::{fence, Ordering},
    time::Duration,
};

use anyhow::{anyhow, Result};
use log::{error, info};
use tokio::{select, task::JoinHandle, time::sleep};
use xenevtchn::EventChannel;
use xengnt::{sys::GrantRef, GrantTab, MappedMemory};
use xenplatform::sys::XEN_PAGE_SHIFT;
use xenstore::{XsdClient, XsdInterface};

#[async_trait::async_trait]
pub trait Fs9pRequestHandler {
    async fn handle(&self, domid: u32, request: Vec<u8>) -> Result<Vec<u8>>;
}

pub struct Fs9pService<RequestHandler: Fs9pRequestHandler + Clone + Sync + Send + 'static> {
    backends: HashMap<u32, Fs9pBackend>,
    evtchn: EventChannel,
    store: XsdClient,
    gnttab: GrantTab,
    handler: RequestHandler,
}

impl<RequestHandler: Fs9pRequestHandler + Clone + Sync + Send + 'static>
    Fs9pService<RequestHandler>
{
    pub async fn new(handler: RequestHandler) -> Result<Fs9pService<RequestHandler>> {
        Ok(Fs9pService {
            backends: HashMap::new(),
            evtchn: EventChannel::open().await?,
            store: XsdClient::open().await?,
            gnttab: GrantTab::open()?,
            handler,
        })
    }

    pub async fn launch(mut self) -> Result<JoinHandle<()>> {
        Ok(tokio::task::spawn(async move {
            if let Err(error) = self.process().await {
                error!("9p processor failed: {}", error);
            }
        }))
    }

    async fn process(&mut self) -> Result<()> {
        self.scan_all_backends().await?;
        let mut watch_handle = self
            .store
            .create_watch("/local/domain/0/backend/9pfs")
            .await?;
        self.store.bind_watch(&watch_handle).await?;
        loop {
            select! {
                x = watch_handle.receiver.recv() => match x {
                    Some(_) => {
                        self.scan_all_backends().await?;
                    }

                    None => {
                        break;
                    }
                },
            }
        }
        Ok(())
    }

    async fn ensure_backend_exists(&mut self, domid: u32, id: u32, path: String) -> Result<()> {
        if self.backends.contains_key(&domid) {
            return Ok(());
        }
        let Some(frontend_path) = self.store.read_string(format!("{}/frontend", path)).await?
        else {
            return Ok(());
        };

        let backend = Fs9pBackend::new::<RequestHandler>(
            path.clone(),
            frontend_path.clone(),
            domid,
            id,
            self.store.clone(),
            self.evtchn.clone(),
            self.gnttab.clone(),
            self.handler.clone(),
        )
        .await?;
        self.backends.insert(domid, backend);
        Ok(())
    }

    async fn scan_all_backends(&mut self) -> Result<()> {
        let domains = self.store.list("/local/domain/0/backend/9pfs").await?;
        let mut seen: Vec<u32> = Vec::new();
        for domid_string in &domains {
            let domid = domid_string.parse::<u32>()?;
            let domid_path = format!("/local/domain/0/backend/9pfs/{}", domid);
            for id_string in self.store.list(&domid_path).await? {
                let id = id_string.parse::<u32>()?;
                let fs9p_path = format!(
                    "/local/domain/0/backend/9pfs/{}/{}",
                    domid_string, id_string
                );
                self.ensure_backend_exists(domid, id, fs9p_path).await?;
                seen.push(domid);
            }
        }

        let mut gone: Vec<u32> = Vec::new();
        for backend in self.backends.keys() {
            if !seen.contains(backend) {
                gone.push(*backend);
            }
        }

        for item in gone {
            if let Some(backend) = self.backends.remove(&item) {
                drop(backend);
            }
        }

        Ok(())
    }
}

pub struct Fs9pBackend {
    pub domid: u32,
    pub id: u32,
    task: JoinHandle<()>,
}

impl Fs9pBackend {
    #[allow(clippy::too_many_arguments)]
    pub async fn new<RequestHandler: Fs9pRequestHandler + Clone + Sync + Sync + Send + 'static>(
        backend: String,
        frontend: String,
        domid: u32,
        id: u32,
        store: XsdClient,
        evtchn: EventChannel,
        gnttab: GrantTab,
        handler: RequestHandler,
    ) -> Result<Fs9pBackend> {
        let processor = Fs9pBackendProcessor::<RequestHandler> {
            backend,
            frontend,
            domid,
            id,
            store,
            handler,
            evtchn,
            gnttab,
        };

        let task = processor.launch().await?;
        Ok(Fs9pBackend { domid, id, task })
    }
}

impl Drop for Fs9pBackend {
    fn drop(&mut self) {
        self.task.abort();
        info!(
            "destroyed 9p backend for domain {} id {}",
            self.domid, self.id
        );
    }
}

#[derive(Clone)]
pub struct Fs9pBackendProcessor<RequestHandler: Fs9pRequestHandler + Clone + Sync + Send + 'static>
{
    backend: String,
    frontend: String,
    id: u32,
    domid: u32,
    store: XsdClient,
    handler: RequestHandler,
    evtchn: EventChannel,
    gnttab: GrantTab,
}

impl<RequestHandler: Fs9pRequestHandler + Clone + Send + Sync + 'static>
    Fs9pBackendProcessor<RequestHandler>
{
    async fn init(&self) -> Result<()> {
        self.store
            .write_string(format!("{}/versions", self.backend), "1")
            .await?;
        self.store
            .write_string(format!("{}/max-rings", self.backend), "4")
            .await?;
        self.store
            .write_string(format!("{}/max-ring-page-order", self.backend), "2")
            .await?;
        self.store
            .write_string(format!("{}/state", self.backend), "2")
            .await?;
        info!(
            "created 9p backend for domain {} id {}",
            self.domid, self.id
        );
        Ok(())
    }

    async fn on_frontend_state_change(&self) -> Result<bool> {
        let state = self
            .store
            .read_string(format!("{}/state", self.frontend))
            .await?
            .unwrap_or("0".to_string())
            .parse::<u32>()?;
        if state == 3 {
            return Ok(true);
        }
        Ok(false)
    }

    async fn on_self_state_change(&self) -> Result<bool> {
        let state = self
            .store
            .read_string(format!("{}/state", self.backend))
            .await?
            .unwrap_or("0".to_string())
            .parse::<u32>()?;
        if state == 5 {
            return Ok(true);
        }
        Ok(false)
    }

    async fn launch(&self) -> Result<JoinHandle<()>> {
        let owned = self.clone();
        Ok(tokio::task::spawn(async move {
            if let Err(error) = owned.processor().await {
                error!("failed to process 9p backend: {}", error);
            }
            let _ = owned
                .store
                .write_string(format!("{}/state", owned.backend), "6")
                .await;
        }))
    }

    async fn processor(&self) -> Result<()> {
        self.init().await?;
        let mut frontend_state_change = self
            .store
            .create_watch(format!("{}/state", self.frontend))
            .await?;
        self.store.bind_watch(&frontend_state_change).await?;
        let rings: Vec<(u64, u32)> = loop {
            match frontend_state_change.receiver.recv().await {
                Some(_) => {
                    if !self.on_frontend_state_change().await? {
                        continue;
                    }

                    let Some(num_rings) = self
                        .store
                        .read_string(format!("{}/num-rings", self.frontend))
                        .await?
                    else {
                        continue;
                    };
                    let Some(num_rings) = num_rings.parse::<u32>().ok() else {
                        continue;
                    };

                    if num_rings > 4 {
                        return Err(anyhow!("max-rings exceeded"));
                    }

                    let mut rings = Vec::new();
                    for i in 0..num_rings {
                        let Some(ring_ref) = self
                            .store
                            .read_string(format!("{}/ring-ref{}", self.frontend, i))
                            .await?
                        else {
                            return Err(anyhow!("ring-ref {} missing", i));
                        };
                        let Some(ring_ref) = ring_ref.parse::<u64>().ok() else {
                            return Err(anyhow!("ring-ref {} invalid", i));
                        };
                        let Some(evtchn) = self
                            .store
                            .read_string(format!("{}/event-channel-{}", self.frontend, i))
                            .await?
                        else {
                            return Err(anyhow!("event-channel {} missing", i));
                        };
                        let Some(evtchn) = evtchn.parse::<u32>().ok() else {
                            return Err(anyhow!("event-channel {} invalid", i));
                        };
                        rings.push((ring_ref, evtchn));
                    }

                    break rings;
                }

                None => {
                    return Ok(());
                }
            }
        };

        let mut tasks = Vec::new();
        for (ring_ref, evtchn) in rings {
            let processor = self.clone();
            let task = tokio::task::spawn(async move {
                if let Err(error) = processor.process_ring(ring_ref, evtchn).await {
                    error!(
                        "failed to process ring {} for domain {} id {}: {}",
                        ring_ref, processor.domid, processor.id, error
                    );
                }
            });
            tasks.push(task);
        }

        let _abort_tasks_guard = scopeguard::guard(tasks, |tasks| {
            for task in tasks {
                task.abort();
            }
        });

        let mut self_state_change = self
            .store
            .create_watch(format!("{}/state", self.backend))
            .await?;
        self.store.bind_watch(&self_state_change).await?;
        self.store
            .write_string(format!("{}/state", self.backend), "4")
            .await?;
        loop {
            select! {
                x = self_state_change.receiver.recv() => match x {
                    Some(_) => {
                        match self.on_self_state_change().await {
                            Err(error) => {
                                error!("failed to process state change for domain {} id {}: {}", self.domid, self.id, error);
                            },

                            Ok(stop) => {
                                if stop {
                                    break;
                                }
                            }
                        }
                    },

                    None => {
                        break;
                    }
                },
            }
        }
        Ok(())
    }

    async fn process_ring(&self, ring_ref: u64, evtchn: u32) -> Result<()> {
        let memory = self
            .gnttab
            .map_grant_refs(
                vec![GrantRef {
                    domid: self.domid,
                    reference: ring_ref as u32,
                }],
                true,
                true,
            )
            .map_err(|e| {
                anyhow!(
                    "failed to map grant ref {} for domid {}: {}",
                    ring_ref,
                    self.domid,
                    e
                )
            })?;

        let ring_order = unsafe { (*(memory.ptr() as *mut Xen9pfsInterface)).ring_order };
        let max_array_size = ((1usize << ring_order) << XEN_PAGE_SHIFT) / 2;
        let pages = max_array_size >> XEN_PAGE_SHIFT;
        let refs = unsafe { (*(memory.ptr() as *mut Xen9pfsInterface)).refs[0..pages].to_vec() }
            .into_iter()
            .map(|grant| GrantRef {
                domid: self.domid,
                reference: grant,
            })
            .collect::<Vec<GrantRef>>();
        let data_memory = self.gnttab.map_grant_refs(refs, true, true).map_err(|e| {
            anyhow!(
                "failed to map data grant refs for domid {}: {}",
                self.domid,
                e
            )
        })?;

        let mut channel = self.evtchn.bind(self.domid, evtchn).await?;
        channel.unmask_sender.send(channel.local_port).await?;
        loop {
            select! {
                _ = channel.receiver.recv() => {},
                _ = sleep(Duration::from_secs(20)) => {}
            };

            let data = unsafe {
                self.read_request_buffer(channel.local_port, &memory, &data_memory, max_array_size)
                    .await?
            };
            println!("ring {} data for request: {:?}", ring_ref, data);
        }
    }

    async unsafe fn read_request_buffer<'a>(
        &self,
        local_port: u32,
        interface_memory: &MappedMemory<'a>,
        data_memory: &MappedMemory<'a>,
        max_size: usize,
    ) -> Result<Vec<u8>> {
        let interface = interface_memory.ptr() as *mut Xen9pfsInterface;
        let data_ptr = (data_memory.ptr() as *mut u8).add(max_size + 128); // i don't really get why i needed to do this
        let data_slice = std::slice::from_raw_parts(data_ptr, max_size - 128);
        for (i, c) in data_slice.iter().enumerate() {
            if *c != 0 {
                println!("FIRST BYTE POSITION: {:?}", i);
                break;
            }
        }
        let cons = (*interface).out_cons;
        let prod = (*interface).out_prod;
        fence(Ordering::Release);
        let size = prod.wrapping_sub(cons);
        let mut data: Vec<u8> = Vec::new();
        if size == 0 || size as usize > max_size {
            return Ok(data);
        }

        if cons < prod {
            data.extend_from_slice(&data_slice[cons as usize..size as usize]);
        } else if size as usize > max_size - cons as usize {
            data.extend_from_slice(&data_slice[cons as usize..max_size - cons as usize]);
            data.extend_from_slice(
                &data_slice
                    [0..size.wrapping_sub(max_size.wrapping_sub(cons as usize) as u32) as usize],
            );
        } else {
            data.extend_from_slice(&data_slice[0..size as usize]);
        }
        fence(Ordering::AcqRel);
        (*interface).out_cons = cons.wrapping_add(size) & (max_size as u32 - 1);
        self.evtchn.notify(local_port).await?;
        Ok(data)
    }
}

#[repr(C)]
#[derive(Debug)]
struct Xen9pfsInterface {
    pub in_cons: u32,
    pub in_prod: u32,
    pub pad1: [u8; 56],
    pub out_cons: u32,
    pub out_prod: u32,
    pub pad2: [u8; 56],
    pub ring_order: u32,
    pub refs: [u32; 4],
}
