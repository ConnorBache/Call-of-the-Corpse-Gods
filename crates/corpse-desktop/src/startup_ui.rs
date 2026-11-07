//! One bounded pure-computation worker. Cancelled results can never replace a run.
use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};
pub struct Loader<T> {
    receiver: Option<Receiver<Result<T, String>>>,
    accept: bool,
}
impl<T> Default for Loader<T> {
    fn default() -> Self { Self { receiver: None, accept: false } }
}
impl<T: Send + 'static> Loader<T> {
    pub fn busy(&self) -> bool { self.receiver.is_some() }
    pub fn start(&mut self, build: impl FnOnce() -> Result<T, String> + Send + 'static) -> bool {
        if self.busy() { return false; }
        let (tx, rx) = sync_channel(1);
        self.receiver = Some(rx);
        self.accept = true;
        std::thread::spawn(move || { let _ = tx.send(build()); });
        true
    }
    pub fn cancel(&mut self) { self.accept = false; }
    /// O(1) queue polling per UI frame; no blocking wait and no partial result.
    /// Cancellation retains one in-flight job until it finishes, bounding memory/work.
    pub fn poll(&mut self, form_active: bool) -> Option<Result<T, String>> {
        if !form_active { self.cancel(); }
        let result = match self.receiver.as_ref()?.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err("Run preparation stopped before completing.".into()),
        };
        self.receiver = None;
        if self.accept { Some(result) } else { None }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeat_start_is_bounded_and_cancelled_result_never_commits() {
        let (tx, rx) = sync_channel(1);
        let mut loader = Loader { receiver: Some(rx), accept: true };
        assert!(!loader.start(|| Ok(99)));
        loader.cancel();
        assert!(loader.busy());
        tx.send(Ok(42)).unwrap();
        assert_eq!(loader.poll(true), None);
        assert!(!loader.busy());
        assert_eq!(loader.poll(true), None);
    }
    #[test]
    fn leaving_form_invalidates_success_even_after_reopening() {
        let (tx, rx) = sync_channel(1);
        let mut loader = Loader { receiver: Some(rx), accept: true };
        assert_eq!(loader.poll(false), None);
        tx.send(Ok(42)).unwrap();
        assert_eq!(loader.poll(true), None);
        assert!(!loader.busy());
    }
    #[test]
    fn accepted_error_is_reported_once_and_allows_retry() {
        let (tx, rx) = sync_channel::<Result<u32, String>>(1);
        let mut loader = Loader { receiver: Some(rx), accept: true };
        tx.send(Err("Invalid floor".into())).unwrap();
        assert_eq!(loader.poll(true), Some(Err("Invalid floor".into())));
        assert!(!loader.busy());
        assert_eq!(loader.poll(true), None);
    }
    #[test]
    fn valid_success_is_delivered_once_and_disconnection_is_visible() {
        let (tx, rx) = sync_channel(1);
        let mut loader = Loader { receiver: Some(rx), accept: true };
        tx.send(Ok(42)).unwrap();
        assert_eq!(loader.poll(true), Some(Ok(42)));
        assert_eq!(loader.poll(true), None);
        let (tx, rx) = sync_channel::<Result<u32, String>>(1);
        let mut loader = Loader { receiver: Some(rx), accept: true };
        drop(tx);
        assert!(loader.poll(true).unwrap().is_err());
        assert!(!loader.busy());
    }
}
