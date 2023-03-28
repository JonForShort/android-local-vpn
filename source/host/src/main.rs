// This is free and unencumbered software released into the public domain.
//
// Anyone is free to copy, modify, publish, use, compile, sell, or
// distribute this software, either in source code form or as a compiled
// binary, for any purpose, commercial or non-commercial, and by any
// means.
//
// In jurisdictions that recognize copyright laws, the author or authors
// of this software dedicate any and all copyright interest in the
// software to the public domain. We make this dedication for the benefit
// of the public at large and to the detriment of our heirs and
// successors. We intend this dedication to be an overt act of
// relinquishment in perpetuity of all present and future rights to this
// software under copyright law.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES OR
// OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
// ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
// OTHER DEALINGS IN THE SOFTWARE.
//
// For more information, please refer to <https://unlicense.org>

use clap::Parser;
use core::tun;
use core::tun_callbacks;
use env_logger::Env;
use once_cell::sync::OnceCell;
use smoltcp::phy::{Medium, TunTapInterface};
use std::ffi::CString;
use std::os::unix::io::AsRawFd;

static OUT_INTERFACE: OnceCell<CString> = OnceCell::new();

/// Tunnel traffic through sockets.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the tun interface.
    #[arg(short, long)]
    tun: String,

    /// Name of the output interface.
    #[arg(short, long)]
    out: String,
}

fn main() {
    let environment = Env::default().default_filter_or("info");
    env_logger::Builder::from_env(environment).init();

    let args = Args::parse();

    OUT_INTERFACE.set(CString::new(args.out).unwrap()).unwrap();

    tun_callbacks::set_socket_created_callback(Some(on_socket_created));

    let tun_name = &args.tun;
    match TunTapInterface::new(tun_name, Medium::Ip) {
        Ok(tun) => {
            set_panic_handler();

            tun::create();
            tun::start(tun.as_raw_fd());

            println!("Press any key to exit");
            std::io::stdin().read_line(&mut String::new()).unwrap();

            tun::stop();
            tun::destroy();

            remove_panic_handler();
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("failed to attach to tun {:?}; permission denied", tun_name);
        }
        Err(_) => {
            eprintln!("failed to attach to tun {:?}", tun_name);
        }
    }
}

fn on_socket_created(socket: i32) {
    bind_socket_to_interface(socket, OUT_INTERFACE.get().unwrap());
}

fn bind_socket_to_interface(socket: i32, interface: &CString) {
    let result = unsafe {
        libc::setsockopt(
            socket,
            libc::SOL_SOCKET,
            libc::SO_BINDTODEVICE,
            interface.as_ptr() as *const libc::c_void,
            std::mem::size_of::<CString>() as libc::socklen_t,
        )
    };
    if result == -1 {
        let error_code = unsafe { *libc::__errno_location() };
        let error: std::io::Result<libc::c_int> = Err(std::io::Error::from_raw_os_error(error_code));
        eprint!("failed to bind socket to interface, error={:?}", error);
    }
}

fn set_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("PANIC [{:?}]", panic_info);
    }));
}

fn remove_panic_handler() {
    let _ = std::panic::take_hook();
}
