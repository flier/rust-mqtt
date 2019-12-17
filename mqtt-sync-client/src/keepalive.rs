use std::time::Duration;

use anyhow::Result;
use timer::{Guard, Timer};

use crate::{
    io::{Receiver, Sender, TryClone},
    packet::Packet,
};

pub struct KeepAlive<T> {
    stream: T,
    timer: Timer,
    delay: Option<time::Duration>,
    guard: Option<Guard>,
}

impl<T> KeepAlive<T>
where
    T: 'static + Sender + TryClone + Send,
{
    pub fn new(stream: T, timeout: Option<Duration>) -> Self {
        let mut keepalive = KeepAlive {
            stream,
            timer: Timer::new(),
            delay: timeout.map(|d| time::Duration::from_std(d).expect("timeout")),
            guard: None,
        };

        keepalive.reschedule_ping();
        keepalive
    }

    fn reschedule_ping(&mut self) {
        self.guard = self.delay.map(|delay| {
            let mut stream = self.stream.try_clone().expect("stream");

            self.timer.schedule_repeating(delay, move || {
                let res = stream.send(Packet::Ping);

                match res {
                    Ok(_) => trace!("send ping @ {}", time::now().ctime()),
                    Err(err) => debug!("send ping failed, {:?}", err),
                }
            })
        });
    }
}

impl<R> Receiver for KeepAlive<R>
where
    R: Receiver,
{
    fn receive(&mut self) -> Result<Packet> {
        self.stream.receive()
    }
}

impl<W> Sender for KeepAlive<W>
where
    W: 'static + Sender + TryClone + Send,
{
    fn send<'a, P: Into<Packet<'a>>>(&mut self, packet: P) -> Result<()> {
        self.stream.send(packet)?;
        self.reschedule_ping();

        Ok(())
    }
}
