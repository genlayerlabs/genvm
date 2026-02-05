use std::{
    os::fd::{AsRawFd, FromRawFd},
    pin::Pin,
};

/// A trait alias for async bidirectional streams
pub trait Stream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send {}

impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send> Stream for T {}

/// Sets a file descriptor to non-blocking mode
pub fn set_fd_nonblocking(fd: std::os::fd::RawFd) {
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL, 0);
        libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
}

/// Async wrapper around a raw file descriptor that implements AsyncRead and AsyncWrite
pub struct AsyncCustomFD(tokio::io::unix::AsyncFd<std::os::fd::OwnedFd>);

impl AsyncCustomFD {
    /// Create a new AsyncCustomFD from an OwnedFd
    /// The fd should already be set to non-blocking mode
    pub fn new(fd: std::os::fd::OwnedFd) -> std::io::Result<Self> {
        Ok(Self(tokio::io::unix::AsyncFd::new(fd)?))
    }

    /// Create a new AsyncCustomFD from a raw fd, taking ownership
    /// Sets the fd to non-blocking mode automatically
    pub unsafe fn from_raw_fd(fd: std::os::fd::RawFd) -> std::io::Result<Self> {
        set_fd_nonblocking(fd);
        let owned = std::os::fd::OwnedFd::from_raw_fd(fd);
        Self::new(owned)
    }
}

impl tokio::io::AsyncRead for AsyncCustomFD {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        loop {
            let mut guard = std::task::ready!(self.0.poll_read_ready(cx))?;

            let unfilled = buf.initialize_unfilled();
            match guard.try_io(|inner| {
                let fd = inner.get_ref().as_raw_fd();
                let result =
                    unsafe { libc::read(fd, unfilled.as_mut_ptr().cast(), unfilled.len()) };
                if result == -1 {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(result as usize)
                }
            }) {
                Ok(Ok(len)) => {
                    buf.advance(len);
                    return std::task::Poll::Ready(Ok(()));
                }
                Ok(Err(err)) => return std::task::Poll::Ready(Err(err)),
                Err(_would_block) => continue,
            }
        }
    }
}

impl tokio::io::AsyncWrite for AsyncCustomFD {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        use std::task::Poll::{Pending, Ready};

        loop {
            let mut ready = match self.0.poll_write_ready(cx) {
                Ready(x) => x?,
                Pending => return Pending,
            };

            let ret = unsafe { libc::write(self.0.as_raw_fd(), buf.as_ptr().cast(), buf.len()) };

            return if ret < 0 {
                let e = std::io::Error::last_os_error();
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    ready.clear_ready();
                    continue;
                } else {
                    Ready(Err(e))
                }
            } else {
                Ready(Ok(ret as usize))
            };
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}

/// A bidirectional stream composed of separate read and write file descriptors
pub struct FdPairStream {
    source: AsyncCustomFD,
    sink: AsyncCustomFD,
}

impl FdPairStream {
    /// Create a new FdPairStream from two OwnedFds
    /// source_fd is used for reading, sink_fd is used for writing
    pub fn new(
        source_fd: std::os::fd::OwnedFd,
        sink_fd: std::os::fd::OwnedFd,
    ) -> std::io::Result<Self> {
        Ok(Self {
            source: AsyncCustomFD::new(source_fd)?,
            sink: AsyncCustomFD::new(sink_fd)?,
        })
    }

    /// Create a new FdPairStream from raw file descriptors
    /// source_fd is used for reading, sink_fd is used for writing
    /// Takes ownership and sets both to non-blocking mode
    pub unsafe fn from_raw_fds(
        source_fd: std::os::fd::RawFd,
        sink_fd: std::os::fd::RawFd,
    ) -> std::io::Result<Self> {
        set_fd_nonblocking(source_fd);
        set_fd_nonblocking(sink_fd);
        let source = std::os::fd::OwnedFd::from_raw_fd(source_fd);
        let sink = std::os::fd::OwnedFd::from_raw_fd(sink_fd);
        Self::new(source, sink)
    }
}

impl tokio::io::AsyncRead for FdPairStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Pin::new(&mut self.source).poll_read(cx, buf)
    }
}

impl tokio::io::AsyncWrite for FdPairStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        Pin::new(&mut self.sink).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Pin::new(&mut self.sink).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Pin::new(&mut self.sink).poll_shutdown(cx)
    }
}
