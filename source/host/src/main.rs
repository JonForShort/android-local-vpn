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

use core::tun;
use env_logger::Env;
use smoltcp::phy::{Medium, TunTapInterface};
use std::os::unix::io::AsRawFd;

fn main() {
    let environment = Env::default().default_filter_or("info");
    env_logger::Builder::from_env(environment).init();

    let matches = clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about("Tunnel to socket.")
        .arg(
            clap::Arg::with_name("tun")
                .short('t')
                .long("tun")
                .value_name("TUN")
                .help("Name of the tun interface")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    let tun_name = matches.value_of("tun").unwrap();
    match TunTapInterface::new(tun_name, Medium::Ip) {
        Ok(tun) => {
            let file_descriptor = tun.as_raw_fd();

            tun::create();
            tun::start(file_descriptor);

            println!("Press any key to exit");
            std::io::stdin().read_line(&mut String::new()).unwrap();

            tun::stop();
            tun::destroy();
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("failed to attach to tun {:?}; permission denied", tun_name);
        }
        Err(_) => {
            eprintln!("failed to attach to tun {:?}", tun_name);
        }
    }
}
