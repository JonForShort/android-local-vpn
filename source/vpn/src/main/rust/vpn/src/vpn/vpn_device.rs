pub mod vpn_device {

    extern crate smoltcp;

    use smoltcp::phy::{ChecksumCapabilities, Device, DeviceCapabilities, Medium, RawSocket};

    struct VpnDevice {
        socket: RawSocket,
    }

    impl<'a> Device<'a> for VpnDevice {
        type RxToken = <RawSocket as Device<'a>>::RxToken;
        type TxToken = <RawSocket as Device<'a>>::TxToken;

        fn capabilities(&self) -> DeviceCapabilities {
            let mut capabilities = DeviceCapabilities::default();
            capabilities.max_transmission_unit = 1500;
            capabilities.max_burst_size = None;
            capabilities.medium = Medium::Ip;
            capabilities.checksum = ChecksumCapabilities::default();
            return capabilities;
        }

        fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
            return self.socket.receive();
        }

        fn transmit(&'a mut self) -> Option<Self::TxToken> {
            return self.socket.transmit();
        }
    }
}
