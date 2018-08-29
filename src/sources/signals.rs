//! Event source for tracking Unix signals
//!
//! Only available on `#[cfg(unix)]`.
//!
//! This allows you to track  and receive Unix signals through the event loop
//! rather than by registering signal handlers. It uses `signalfd` under the hood.
//!
//! The source will take care of masking and unmasking signals for the thread it runs on,
//! but you are responsible for masking them on other threads if you run them. The simplest
//! way to ensure that is to setup the signal event source before spawning any thread, as
//! they'll inherit their parent signal mask.

use std::cell::RefCell;
use std::io;
use std::os::raw::c_int;
use std::os::unix::io::AsRawFd;
use std::rc::Rc;

use mio::{Evented, Poll, PollOpt, Ready, Token};

use nix::sys::signal::SigSet;
pub use nix::sys::signal::Signal;
pub use nix::sys::signalfd::siginfo;
use nix::sys::signalfd::{SfdFlags, SignalFd};

use {EventDispatcher, EventSource};

/// An event generated by the signal event source
#[derive(Copy,Clone)]
pub struct Event {
    info: siginfo,
}

impl Event {
    /// Retrieve the signal number that was receive
    pub fn signal(&self) -> Signal {
        Signal::from_c_int(self.info.ssi_signo as c_int).unwrap()
    }

    /// Access the full `siginfo_t` associated with this signal event
    pub fn full_info(&self) -> siginfo {
        self.info
    }
}

/// An event source for receiving Unix signals
pub struct Signals {
    sfd: Rc<RefCell<SignalFd>>,
    mask: SigSet,
}

impl Signals {
    /// Create a new signal event source listening on the specified list of signals
    pub fn new(signals: &[Signal]) -> io::Result<Signals> {
        let mut mask = SigSet::empty();
        for &s in signals {
            mask.add(s);
        }

        // Mask the signals for this thread
        mask.thread_block().map_err(no_nix_err)?;
        // Create the SignalFd
        let sfd = SignalFd::with_flags(&mask, SfdFlags::SFD_NONBLOCK | SfdFlags::SFD_CLOEXEC)
            .map_err(no_nix_err)?;

        Ok(Signals {
            sfd: Rc::new(RefCell::new(sfd)),
            mask,
        })
    }

    /// Add a list of signals to the signals source
    ///
    /// If this function returns an error, the signal mask of the thread may
    /// have still been changed.
    pub fn add_signals(&mut self, signals: &[Signal]) -> io::Result<()> {
        for &s in signals {
            self.mask.add(s);
        }
        self.mask.thread_block().map_err(no_nix_err)?;
        self.sfd
            .borrow_mut()
            .set_mask(&self.mask)
            .map_err(no_nix_err)?;
        Ok(())
    }

    /// Remove a list of signals to the signals source
    ///
    /// If this function returns an error, the signal mask of the thread may
    /// have still been changed.
    pub fn remove_signals(&mut self, signals: &[Signal]) -> io::Result<()> {
        let mut removed = SigSet::empty();
        for &s in signals {
            self.mask.remove(s);
            removed.add(s);
        }
        removed.thread_unblock().map_err(no_nix_err)?;
        self.sfd
            .borrow_mut()
            .set_mask(&self.mask)
            .map_err(no_nix_err)?;
        Ok(())
    }

    /// Replace the list of signals of the source
    ///
    /// If this function returns an error, the signal mask of the thread may
    /// have still been changed.
    pub fn set_signals(&mut self, signals: &[Signal]) -> io::Result<()> {
        let mut new_mask = SigSet::empty();
        for &s in signals {
            new_mask.add(s);
        }

        self.mask.thread_unblock().map_err(no_nix_err)?;
        new_mask.thread_block().map_err(no_nix_err)?;
        self.sfd
            .borrow_mut()
            .set_mask(&new_mask)
            .map_err(no_nix_err)?;
        self.mask = new_mask;

        Ok(())
    }
}

impl Drop for Signals {
    fn drop(&mut self) {
        // we cannot handle error here
        if let Err(e) = self.mask.thread_unblock() {
            eprintln!("[calloop] Failed to unmask signals: {:?}", e);
        }
    }
}

fn no_nix_err(err: ::nix::Error) -> io::Error {
    match err {
        ::nix::Error::Sys(errno) => errno.into(),
        _ => unreachable!(),
    }
}

impl Evented for Signals {
    fn register(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        ::mio::unix::EventedFd(&self.sfd.borrow().as_raw_fd()).register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        ::mio::unix::EventedFd(&self.sfd.borrow().as_raw_fd())
            .reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        ::mio::unix::EventedFd(&self.sfd.borrow().as_raw_fd()).deregister(poll)
    }
}

impl EventSource for Signals {
    type Event = Event;

    fn interest(&self) -> Ready {
        Ready::readable()
    }

    fn pollopts(&self) -> PollOpt {
        PollOpt::edge()
    }

    fn make_dispatcher<F: FnMut(Event) + 'static>(
        &self,
        callback: F,
    ) -> Rc<RefCell<EventDispatcher>> {
        Rc::new(RefCell::new(Dispatcher {
            callback,
            sfd: self.sfd.clone(),
        }))
    }
}

struct Dispatcher<F: FnMut(Event) + 'static> {
    callback: F,
    sfd: Rc<RefCell<SignalFd>>,
}

impl<F: FnMut(Event) + 'static> EventDispatcher for Dispatcher<F> {
    fn ready(&mut self, _: Ready) {
        loop {
            let ret = self.sfd.borrow_mut().read_signal();
            match ret {
                Ok(Some(info)) => (self.callback)(Event { info }),
                Ok(None) => {
                    // nothing more to read
                    break;
                }
                Err(e) => {
                    eprintln!("[calloop] Error reading from signalfd: {:?}", e);
                    break;
                }
            }
        }
    }
}
