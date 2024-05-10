use clap::Parser;
use hex;
use pnet::datalink::Channel::Ethernet;

use std::sync::{Arc, Mutex, RwLock};
use std::{thread, time};

// Command line args parsing struct (CLAP)
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Packet as hex string
    #[arg(short, long, value_name = "PACKET")]
    packet: String,

    /// Interface name
    #[arg(short, long, value_name = "INTERFACE")]
    interface_name: String,

    /// Number of threads
    #[arg(
        short = 't',
        long = "threads",
        value_name = "TREADS_NUMBER",
        default_value_t = 1
    )]
    threads_number: usize,

    /// Number of seconnds to pause before counters output
    #[arg(
        short = 's',
        long = "sleep",
        value_name = "SLEEP_SECS",
        default_value_t = 1
    )]
    sleep: usize,
}

fn main() {
    // TODO proper debug

    let cli = Cli::parse();

    // TODO decoding hex string to [u8] without external crates
    let packet = hex::decode(&cli.packet).expect(&format!(
        "Can not decode to hex packet string: \"{}\"",
        &cli.packet
    ));
    let packet_length = packet.len();

    let interface = pnet::datalink::interfaces()
        .into_iter()
        .filter(|interface| interface.name == cli.interface_name)
        .next()
        .expect(&format!("Not found interface name {}", cli.interface_name));

    let packet_ref = Arc::new(RwLock::new(packet));
    let interface_ref = Arc::new(RwLock::new(interface));

    let mut counters: Vec<Arc<Mutex<(usize, usize)>>> = Vec::new();

    for thread_number in 0..cli.threads_number {
        let interface_ref = Arc::clone(&interface_ref);
        let packet_ref = Arc::clone(&packet_ref);

        counters.push(Arc::new(Mutex::new((0, 0))));
        let counter_ref = Arc::clone(&counters[thread_number]);

        thread::spawn(move || {
            let interface = interface_ref.read().unwrap();
            let packet = packet_ref.read().unwrap();

            // Create a new channel, dealing with layer 2 packets
            let (mut tx, _rx) = match pnet::datalink::channel(&interface, Default::default()) {
                Ok(Ethernet(tx, rx)) => (tx, rx),
                Ok(_) => panic!("Unhandled channel type"),
                Err(e) => panic!(
                    "An error occurred when creating the datalink channel: {}",
                    e
                ),
            };

            let mut ok_counter: usize = 0;
            let mut error_counter: usize = 0;

            loop {
                match tx.send_to(&packet, None).unwrap() {
                    Ok(_) => ok_counter += 1,
                    Err(_) => error_counter += 1,
                }

                *counter_ref.lock().unwrap() = (ok_counter, error_counter);
            }
        });
    }

    let mut ok_sum: usize = 0;
    let mut ok_sum_previous: usize = 0;
    let mut error_sum: usize = 0;
    let mut error_sum_previous: usize = 0;
    loop {
        thread::sleep(time::Duration::from_secs(cli.sleep as u64));
        for counter in &counters {
            ok_sum += counter.lock().unwrap().0;
            error_sum += counter.lock().unwrap().1;
        }
        println!(
            "OK: {}    Errors: {}    Speed: {} Mbps",
            ok_sum - ok_sum_previous,
            error_sum - error_sum_previous,
            packet_length * (ok_sum - ok_sum_previous) * 8 / 1024 / 1024
        );
        ok_sum_previous = ok_sum;
        error_sum_previous = error_sum;
        ok_sum = 0;
        error_sum = 0;
    }
}
