use std::{
    any::{Any, type_name},
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tokio::sync::watch;
use veilid_core::{KeyPair, RecordKey, VeilidValueChange};
use veilid_tools::spawn_detached;

use crate::{
    api::{Document, DocumentError, TypedReference},
    veilid::{RecordPool, updates::UpdateHandler},
};

// ==== WatchRouter ====
// dispatches veilid value_change events to per-record watch channels.
// each record gets a watch::Sender<()>; subscribers get independent Receivers.

pub struct WatchRouter {
    routes: Mutex<HashMap<RecordKey, watch::Sender<()>>>,
}

impl Default for WatchRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl WatchRouter {
    pub fn new() -> Self {
        Self {
            routes: Mutex::new(HashMap::new()),
        }
    }

    // subscribe to change notifications for a record.
    // creates the entry if this is the first subscriber.
    pub(crate) fn subscribe(&self, key: RecordKey) -> watch::Receiver<()> {
        let mut routes = self.routes.lock().unwrap();
        match routes.get(&key) {
            Some(tx) => tx.subscribe(),
            None => {
                let (tx, rx) = watch::channel(());
                routes.insert(key, tx);
                rx
            }
        }
    }

    // atomically removes the entry only if it has no remaining receivers.
    // returns true if deregistered. combining the check and remove under one lock
    // prevents a concurrent open() from inserting a new subscriber between the two.
    // TODO: there is still a narrow race between this returning true and the caller invoking pool.cancel_watch
    // a concurrent open() that called pool.watch() before this point will have its DHT watch silently cancelled
    pub(crate) fn deregister_if_empty(&self, key: &RecordKey) -> bool {
        let mut routes = self.routes.lock().unwrap();
        match routes.get(key) {
            Some(tx) if tx.receiver_count() == 0 => {
                routes.remove(key);
                true
            }
            _ => false,
        }
    }
}

impl UpdateHandler for WatchRouter {
    fn value_change(&self, change: &VeilidValueChange) {
        let routes = self.routes.lock().unwrap();
        if let Some(tx) = routes.get(&change.key) {
            // not sending the actual data here, just a notification which will trigger a re-read in the coordinator
            let _ = tx.send(());
        }
    }
}

// ==== WatchCoordinators ====
// one coordinator task per record. does a single D::read() per veilid notification
// and fans the result out to all subscribers to avoid duplicate reads for multiple subscribers to the same record.

type CoordinatorSender<D> = Arc<watch::Sender<Result<<D as Document>::View, DocumentError>>>;
// keying on record key + document type here to avoid coordinator tasks being overwritten by different document types.
// in practice this should never happen since each record only has one valid type
// but we want to make sure things don't break even if we somehow erroneaously try to open the same record with different types
// (using type_name as opposed to TypeId to avoid a 'static bound on D bubling up all the way to the intersect api)
type CoordinatorMap = Arc<Mutex<HashMap<(RecordKey, &'static str), Box<dyn Any + Send + Sync>>>>;

pub struct WatchCoordinators {
    inner: CoordinatorMap,
}

impl Default for WatchCoordinators {
    fn default() -> Self {
        Self::new()
    }
}

impl WatchCoordinators {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // if a coordinator is already running for this record + document type,
    // returns a new subscriber receiver (immediately holds the latest cached view).
    pub fn try_subscribe<D: Document>(
        &self,
        key: &RecordKey,
    ) -> Option<watch::Receiver<Result<D::View, DocumentError>>> {
        self.inner
            .lock()
            .unwrap()
            .get(&(key.clone(), type_name::<D>()))?
            .downcast_ref::<CoordinatorSender<D>>()
            .map(|s| s.subscribe())
    }

    // creates a coordinator for this record and returns a receiver.
    // if another open() raced us and already created one, subscribes to that instead.
    pub fn create<D: Document>(
        &self,
        typed_ref: TypedReference<D>,
        initial: D::View,
        pool: Arc<RecordPool>,
        identity: Option<KeyPair>,
        notify_rx: watch::Receiver<()>,
        router: Arc<WatchRouter>,
    ) -> watch::Receiver<Result<D::View, DocumentError>> {
        let mut map = self.inner.lock().unwrap();

        // double-check: another open() may have raced us during the async initial read
        if let Some(s) = map
            .get(&(typed_ref.reference().record().clone(), type_name::<D>()))
            .and_then(|b| b.downcast_ref::<CoordinatorSender<D>>())
        {
            return s.subscribe();
        }

        let (tx, rx) = watch::channel(Ok(initial));
        let sender: CoordinatorSender<D> = Arc::new(tx);
        map.insert(
            (typed_ref.reference().record().clone(), type_name::<D>()),
            Box::new(Arc::clone(&sender)),
        );
        drop(map); // drop the lock

        let inner = Arc::clone(&self.inner);
        spawn_detached("intersect-coordinator", async move {
            coordinator_task::<D>(typed_ref, pool, identity, notify_rx, sender, router, inner)
                .await;
        });

        rx
    }
}

async fn coordinator_task<D: Document>(
    typed_ref: TypedReference<D>,
    pool: Arc<RecordPool>,
    identity: Option<KeyPair>,
    mut notify_rx: watch::Receiver<()>,
    sender: CoordinatorSender<D>,
    router: Arc<WatchRouter>,
    coordinators: CoordinatorMap,
) {
    // seed last_view from the initial value already in the channel
    let mut last_view: Option<D::View> = sender.borrow().as_ref().ok().cloned();

    loop {
        if notify_rx.changed().await.is_err() {
            break; // WatchRouter entry dropped
        }

        // notification triggers re-read of the document
        match D::read(&typed_ref, identity.as_ref(), false, &pool).await {
            Ok(new_view) => {
                if Some(&new_view) != last_view.as_ref() {
                    last_view = Some(new_view.clone());
                    if sender.send(Ok(new_view)).is_err() {
                        break; // all receivers dropped
                    }
                }
            }
            Err(e) => {
                // send the error but don't update last_view
                // next successful read will still be compared against the last good view
                if sender.send(Err(e)).is_err() {
                    break;
                }
            }
        }
    }

    // remove from coordinator map, this drops the map's Arc clone.
    // when this function returns, the local sender Arc drops too.
    // once all Arc clones are gone, the watch::Sender drops and any remaining
    // receivers' changed() will return Err.
    coordinators
        .lock()
        .unwrap()
        .remove(&(typed_ref.reference().record().clone(), type_name::<D>()));

    drop(notify_rx); // make sure to decrement listeners before we check if empty
    if router.deregister_if_empty(typed_ref.reference().record()) {
        let _ = pool.cancel_watch(&typed_ref.reference()).await;
    }
}
