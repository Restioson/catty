//! Send a value and asynchronously wait for it.
//!
//! # Example
//! ```rust
//! # pollster::block_on(async {
//! let (tx, rx) = catty::oneshot();
//! tx.send("Hello!");
//! assert_eq!(rx.await, Ok("Hello!"));
//! # })
//! ```
#![doc(html_logo_url = "https://raw.githubusercontent.com/Restioson/catty/master/catty.svg")]

use spin::Mutex;
use std::fmt::{self, Display, Formatter};
use std::task::{Context, Poll, Waker};
use std::{error::Error, future::Future, mem, pin::Pin, sync::Arc};

struct InnerArc<T>(Arc<Mutex<State<T>>>);

impl<T> InnerArc<T> {
    /// Replace the inner state of the channel with a new state given the old state.
    fn replace_state<R>(&self, f: impl FnOnce(State<T>) -> (R, State<T>)) -> R {
        let mut state = self.0.lock();
        let old_state = mem::take(&mut *state);
        let (ret, new_state) = f(old_state);
        *state = new_state;
        ret
    }
}

impl<T> Clone for InnerArc<T> {
    fn clone(&self) -> Self {
        InnerArc(self.0.clone())
    }
}

impl<T> Drop for InnerArc<T> {
    fn drop(&mut self) {
        self.replace_state(|old| match old {
            State::ReceiverWaiting(waker) => (waker.wake(), State::Disconnected),
            State::ItemSent(it) => ((), State::ItemSent(it)),
            _ => ((), State::Disconnected),
        })
    }
}

enum State<T> {
    ReceiverNotYetPolled,
    ReceiverWaiting(Waker),
    ItemSent(T),
    Disconnected,
}

impl<T> Default for State<T> {
    fn default() -> Self {
        State::ReceiverNotYetPolled
    }
}

/// The sender side has disconnected.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct Disconnected;

impl Display for Disconnected {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("oneshot disconnected")
    }
}

impl Error for Disconnected {}

/// The sender side of a oneshot channel, created with [`catty::oneshot`](fn.oneshot.html). A value
/// can be sent into the channel to be waited on by the receiver.
///
/// # Example
///
/// ```rust
/// # pollster::block_on(async {
/// let (tx, rx) = catty::oneshot();
/// tx.send("Hello!");
/// assert_eq!(rx.await, Ok("Hello!"));
/// # })
/// ```
pub struct Sender<T>(InnerArc<T>);

impl<T> Sender<T> {
    /// Sends an item, consuming the sender. This will return the item if the receiver was dropped.
    pub fn send(self, item: T) -> Result<(), T> {
        self.0.replace_state(|old| match old {
            State::Disconnected => (Err(item), State::Disconnected),
            State::ReceiverWaiting(waker) => (Ok(waker.wake()), State::ItemSent(item)),
            _ => (Ok(()), State::ItemSent(item)),
        })
    }
}

/// The receiver side of a oneshot channel, created with [`catty::oneshot`](fn.oneshot.html). This
/// is a future resolving to item, or an error if the sender side has been dropped without sending
/// a message. After a message is received, subsequent polls will return `Err(Disconnected)`.
///
/// # Example
///
/// ```rust
/// # pollster::block_on(async {
/// let (tx, rx) = catty::oneshot();
/// tx.send("Hello!");
/// assert_eq!(rx.await, Ok("Hello!"));
/// # })
/// ```
pub struct Receiver<T>(InnerArc<T>);

impl<T> Receiver<T> {
    /// Checks to see if there is a message waiting in the channel. This will return `None` if there
    /// a message has not been sent, and `Err(Disconnected)` if a message has already been received.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use catty::Disconnected;
    /// # pollster::block_on(async {
    /// let (tx, rx) = catty::oneshot();
    /// tx.send("Hello!");
    /// assert_eq!(rx.try_recv(), Ok(Some("Hello!")));
    /// # })
    /// ```
    ///
    /// Subsequent receives return `Err(Disconnected)`:
    /// ```rust
    /// # use catty::Disconnected;
    /// # pollster::block_on(async {
    /// let (tx, rx) = catty::oneshot();
    /// tx.send("Hello!");
    /// assert_eq!(rx.try_recv(), Ok(Some("Hello!")));
    /// assert!(rx.try_recv().is_err());
    /// assert_eq!(rx.await, Err(Disconnected))
    /// # })
    /// ```
    pub fn try_recv(&self) -> Result<Option<T>, Disconnected> {
        self.0.replace_state(|old| match old {
            State::ItemSent(item) => (Ok(Some(item)), State::Disconnected),
            State::Disconnected => (Err(Disconnected), State::Disconnected),
            old => (Ok(None), old),
        })
    }

    /// Poll this [`Receiver`] for the sent item.
    ///
    /// This function can be called on `&self` and may thus be preferred over the [`Future`]
    /// implementation in some scenarios.
    pub fn poll_recv(&self, cx: &mut Context<'_>) -> Poll<Result<T, Disconnected>> {
        self.0.replace_state(|old| match old {
            State::ItemSent(item) => (Poll::Ready(Ok(item)), State::Disconnected),
            State::Disconnected => (Poll::Ready(Err(Disconnected)), State::Disconnected),
            _ => (Poll::Pending, State::ReceiverWaiting(cx.waker().clone())),
        })
    }
}

impl<T> Future for Receiver<T> {
    type Output = Result<T, Disconnected>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.poll_recv(cx)
    }
}

/// Creates a oneshot channel. A value can be sent into this channel and then be asynchronously
/// waited for.
///
/// # Examples
///
/// Sending a value:
///
/// ```rust
/// # pollster::block_on(async {
/// let (tx, rx) = catty::oneshot();
/// tx.send("Hello!");
/// assert_eq!(rx.await, Ok("Hello!"));
/// # })
/// ```
///
/// Dropping the sender:
///
/// ```rust
/// # use catty::Disconnected;
/// # pollster::block_on(async {
/// let (tx, rx) = catty::oneshot::<u32>();
/// drop(tx);
/// assert_eq!(rx.await, Err(Disconnected));
/// # })
/// ```
///
/// Dropping the receiver:
///
/// ```rust
/// # use catty::Disconnected;
/// let (tx, rx) = catty::oneshot::<&str>();
/// drop(rx);
/// assert_eq!(tx.send("Hello!"), Err("Hello!"));
/// ```
#[inline]
pub fn oneshot<T: Send>() -> (Sender<T>, Receiver<T>) {
    let inner_arc = InnerArc(Arc::new(Mutex::new(State::default())));
    (Sender(inner_arc.clone()), Receiver(inner_arc))
}
