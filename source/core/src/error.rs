#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("smoltcp::socket::tcp::RecvError {0:?}")]
    TcpRecv(#[from] smoltcp::socket::tcp::RecvError),

    #[error("smoltcp::socket::tcp::SendError {0:?}")]
    TcpSend(#[from] smoltcp::socket::tcp::SendError),

    #[error("smoltcp::socket::udp::SendError {0:?}")]
    UdpSend(#[from] smoltcp::socket::udp::SendError),

    #[error("smoltcp::socket::udp::RecvError {0:?}")]
    UdpRecv(#[from] smoltcp::socket::udp::RecvError),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
