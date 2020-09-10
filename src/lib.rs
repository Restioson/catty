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

use spinning_top::{Spinlock, SpinlockGuard};
use std::fmt::{self, Display, Formatter};
use std::task::{Context, Poll, Waker};
use std::{error::Error, future::Future, mem, pin::Pin, sync::Arc};

struct InnerArc<T>(Arc<Spinlock<ChannelState<T>>>);

impl<T> InnerArc<T> {
    fn take_old_state(&self) -> (ChannelState<T>, SpinlockGuard<'_, ChannelState<T>>) {
        let mut state = self.0.lock();
        (mem::take(&mut *state), state)
    }
}

impl<T> Clone for InnerArc<T> {
    fn clone(&self) -> Self {
        InnerArc(self.0.clone())
    }
}

impl<T> Drop for InnerArc<T> {
    fn drop(&mut self) {
        let (old, mut state) = self.take_old_state();

        match old {
            ChannelState::ReceiverWaiting(waker) => waker.wake(),
            ChannelState::ItemSent(it) => {
                *state = ChannelState::ItemSent(it);
                return;
            }
            _ => {}
        }

        *state = ChannelState::SideDropped;
    }
}

enum ChannelState<T> {
    ReceiverNotYetPolled,
    ReceiverWaiting(Waker),
    ItemSent(T),
    SideDropped,
}

impl<T> Default for ChannelState<T> {
    fn default() -> Self {
        ChannelState::ReceiverNotYetPolled
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
pub struct Sender<T>(InnerArc<T>);

impl<T> Sender<T> {
    /// Sends an item, consuming the sender. This will return the item if the receiver was dropped.
    pub fn send(self, item: T) -> Result<(), T> {
        let (old, mut state) = self.0.take_old_state();

        match old {
            ChannelState::SideDropped => return Err(item),
            ChannelState::ReceiverWaiting(waker) => waker.wake(),
            _ => {}
        }

        *state = ChannelState::ItemSent(item);
        Ok(())
    }
}

/// The receiver side of a oneshot channel, created with [`catty::oneshot`](fn.oneshot.html). This
/// is a future resolving to item, or an error if the sender side has been dropped.
pub struct Receiver<T>(InnerArc<T>);

impl<T> Receiver<T> {
    /// Checks to see if there is a message waiting in the channel.
    pub fn try_recv(&mut self) -> Result<Option<T>, Disconnected> {
        let (old, mut state) = self.0.take_old_state();

        match old {
            ChannelState::ItemSent(item) => Ok(Some(item)),
            ChannelState::SideDropped => Err(Disconnected),
            _ => {
                *state = old;
                Ok(None)
            }
        }
    }
}

impl<T> Future for Receiver<T> {
    type Output = Result<T, Disconnected>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (old, mut state) = self.0.take_old_state();

        match old {
            ChannelState::ItemSent(item) => return Poll::Ready(Ok(item)),
            ChannelState::SideDropped => return Poll::Ready(Err(Disconnected)),
            _ => {}
        }

        *state = ChannelState::ReceiverWaiting(cx.waker().clone());
        Poll::Pending
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
#[cfg(not(feature = "nightly"))]
#[inline]
pub fn oneshot<T: Send>() -> (Sender<T>, Receiver<T>) {
    let inner_arc = InnerArc(Arc::new(Spinlock::new(ChannelState::default())));
    (Sender(inner_arc.clone()), Receiver(inner_arc))
}

/// Creates a oneshot channel. A value can be sent into this channel and then be asynchronously
/// waited for.
///
/// # Examples
///
/// Sending a value:
///
/// ```rust
/// let (tx, rx) = catty::oneshot();
/// tx.send("Hello!");
/// assert_eq!(pollster::block_on(rx), Ok("Hello!"));
/// ```
///
/// Dropping the sender:
///
/// ```rust
/// # use catty::Disconnected;
/// let (tx, rx) = catty::oneshot::<u32>();
/// drop(tx);
/// assert_eq!(pollster::block_on(rx), Err(Disconnected));
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
#[cfg(feature = "nightly")]
#[inline]
pub const fn oneshot<T: Send>() -> (Sender<T>, Receiver<T>) {
    let inner_arc = InnerArc(Arc::new(Spinlock::new(ChannelState::default())));
    (Sender(inner_arc.clone()), Receiver(inner_arc))
}
