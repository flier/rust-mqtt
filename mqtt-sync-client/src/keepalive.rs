use std::io;
use std::time::Duration;

use timer::{Guard, Timer};

use crate::{
    io::{ReadExt, TryClone, WriteExt},
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
    T: 'static + WriteExt + TryClone + Send,
{
    pub fn new(stream: T, timeout: Option<Duration>) -> Self {
        let mut keepalive = KeepAlive {
            stream,
            timer: Timer::new(),
            delay: timeout.map(|d| time::Duration::from_std(d).expect("timeout")),
            guard: None,
        };

        keepalive.schedule_keepalive();
        keepalive
    }

    fn schedule_keepalive(&mut self) {
        self.guard = self.delay.map(|delay| {
            let mut stream = self.stream.try_clone().expect("stream");

            self.timer.schedule_with_delay(delay, move || {
                let res = stream.send(Packet::Ping);

                match res {
                    Ok(_) => {}
                    Err(err) => {}
                }
            })
        });
    }
}

impl<R> ReadExt for KeepAlive<R>
where
    R: ReadExt,
{
    fn receive(&mut self) -> io::Result<Packet> {
        self.stream.receive()
    }
}

impl<W> WriteExt for KeepAlive<W>
where
    W: 'static + WriteExt + TryClone + Send,
{
    fn send<'a, P: Into<Packet<'a>>>(&mut self, packet: P) -> io::Result<()> {
        self.stream.send(packet)?;
        self.schedule_keepalive();

        Ok(())
    }
}
