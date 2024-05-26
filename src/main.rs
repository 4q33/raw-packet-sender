use bytemuck::{self, bytes_of};
use clap::Parser;
use hex;
use pnet::datalink::Channel::Ethernet;

use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use std::{thread, time};

// Command line args parsing struct (CLAP)
#[derive(Parser, Debug, Clone)]
#[command(
    author = "4q33",
    version = "0.0.1",
    about = "Small tool for sending raw ethernet packets",
    long_about = r###"Small tool for sending raw ethernet packets
Usage example: 
  raw-packet-sender --packet 00..00 --interface dummy0 --threads 1 --watch 1 --thread-number --packet-number --sleep 1
Where:
  packet - raw hex string of ethernet packet (in the example middle part of 62 zeroes is replaced by "..")
  interface - name of the ethernet inteface to which packets will be sent 
  thread - number of spawned threads (default 1)
  watch - pause in seconds between counters checking
  thread-number - add thread number to the end of packet data
  packet-number - add packet number to the end of packet data (counts only successfully sent packets)
  sleep - insert a sleep pause to sending thread (in milliseconds, minimal value: 0.001)

If activated thread-number and packet-number then thread number will be added before packet number in the way:
  raw packet data + thread number + packet number
Size of thread number and packet number values is usize. Endiannes is inferred from system."###
)]
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

    /// Number of seconds to pause before counters output
    #[arg(
        short ='w',
        long = "watch",
        value_name = "WATCH_SECONDS",
        default_value_t = 1
    )]
    watch: usize,

    /// Add thread number to the end of packet data
    #[arg(long = "thread-number", default_value_t = false)]
    add_thread_number: bool,

    /// Add packet number to the end of packet data
    #[arg(long = "packet-number", default_value_t = false)]
    add_packet_number: bool,

    /// Insert a sleep to sending thread (in milliseconds)
    #[arg(
        short = 's',
        long = "sleep",
        value_name = "SLEEP_MILLISECONDS",
    )]
    sleep: Option<f64>,
}

fn main() {
    // TODO proper debug

    let cli = Cli::parse();

    // TODO decoding hex string to [u8] without external crates
    let packet = hex::decode(&cli.packet).expect(&format!(
        "Can not decode to hex packet string: \"{}\"",
        &cli.packet
    ));

    let packet_length = match (cli.add_thread_number, cli.add_packet_number) {
        (true, true) => packet.len() + usize::BITS as usize / 8,
        (true, false) | (false, true) => packet.len() + usize::BITS as usize / 4,
        (false, false) => packet.len(),
    };

    let sleep_nanos = match cli.sleep{
        Some(x) => Some((x*1_000_000.0) as u64),
        None => None
    };

    let interface = pnet::datalink::interfaces()
        .into_iter()
        .filter(|interface| interface.name == cli.interface_name)
        .next()
        .expect(&format!("Not found interface name {}", cli.interface_name));

    let packet_ref = Arc::new(RwLock::new(packet));
    let interface_ref = Arc::new(RwLock::new(interface));
    let add_thread_number_ref = Arc::new(RwLock::new(cli.add_thread_number));
    let add_packet_number_ref = Arc::new(RwLock::new(cli.add_packet_number));
    let sleep_nanos_ref = Arc::new(RwLock::new(sleep_nanos));
    let mut counters: Vec<Arc<Mutex<(usize, usize)>>> = Vec::new();

    for thread_number in 0..cli.threads_number {
        let interface_ref = Arc::clone(&interface_ref);
        let packet_ref = Arc::clone(&packet_ref);
        let add_thread_number_ref = Arc::clone(&add_thread_number_ref);
        let add_packet_number_ref = Arc::clone(&add_packet_number_ref);
        let sleep_nanos_ref = Arc::clone(&sleep_nanos_ref);

        counters.push(Arc::new(Mutex::new((0, 0))));
        let counter_ref = Arc::clone(&counters[thread_number]);

        thread::spawn(move || -> ! {
            let interface = interface_ref.read().unwrap();
            let packet = packet_ref.read().unwrap();
            let add_thread_number = *add_thread_number_ref.read().unwrap();
            let add_packet_number = *add_packet_number_ref.read().unwrap();
            let sleep_nanos = *sleep_nanos_ref.read().unwrap();
            
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
            if let Some(sleep_nanos) = sleep_nanos {
                let sleep_duration = Duration::from_nanos(sleep_nanos);
                println!("{:?}", sleep_duration);

                // TODO increase performance for thread number adding
                match (add_thread_number, add_packet_number) {
                    (true, true) => {
                        loop {
                            // TODO use usize::be_bytes_of instead of bytemuck crate
                            match tx
                                .send_to(
                                    &[&packet, bytes_of(&thread_number), bytes_of(&ok_counter)]
                                        .concat(),
                                    None,
                                )
                                .unwrap()
                            {
                                Ok(_) => ok_counter += 1,
                                Err(_) => error_counter += 1,
                            }
                            *counter_ref.lock().unwrap() = (ok_counter, error_counter);
                            thread::sleep(sleep_duration);
                        }
                    }

                    (true, false) => loop {
                        match tx
                            .send_to(&[&packet, bytes_of(&thread_number)].concat(), None)
                            .unwrap()
                        {
                            Ok(_) => ok_counter += 1,
                            Err(_) => error_counter += 1,
                        }
                        *counter_ref.lock().unwrap() = (ok_counter, error_counter);
                        thread::sleep(sleep_duration);
                    },

                    (false, true) => loop {
                        match tx
                            .send_to(&[&packet, bytes_of(&ok_counter)].concat(), None)
                            .unwrap()
                        {
                            Ok(_) => ok_counter += 1,
                            Err(_) => error_counter += 1,
                        }
                        *counter_ref.lock().unwrap() = (ok_counter, error_counter);
                        thread::sleep(sleep_duration);
                    },

                    (false, false) => loop {
                        match tx.send_to(&packet, None).unwrap() {
                            Ok(_) => ok_counter += 1,
                            Err(_) => error_counter += 1,
                        }
                        *counter_ref.lock().unwrap() = (ok_counter, error_counter);
                        thread::sleep(sleep_duration);
                    },
                }
            }
            else {
                match (add_thread_number, add_packet_number) {
                    (true, true) => {
                        loop {
                            // TODO use usize::be_bytes_of instead of bytemuck crate
                            match tx
                                .send_to(
                                    &[&packet, bytes_of(&thread_number), bytes_of(&ok_counter)]
                                        .concat(),
                                    None,
                                )
                                .unwrap()
                            {
                                Ok(_) => ok_counter += 1,
                                Err(_) => error_counter += 1,
                            }
                            *counter_ref.lock().unwrap() = (ok_counter, error_counter);
                        }
                    }

                    (true, false) => loop {
                        match tx
                            .send_to(&[&packet, bytes_of(&thread_number)].concat(), None)
                            .unwrap()
                        {
                            Ok(_) => ok_counter += 1,
                            Err(_) => error_counter += 1,
                        }
                        *counter_ref.lock().unwrap() = (ok_counter, error_counter);
                    },

                    (false, true) => loop {
                        match tx
                            .send_to(&[&packet, bytes_of(&ok_counter)].concat(), None)
                            .unwrap()
                        {
                            Ok(_) => ok_counter += 1,
                            Err(_) => error_counter += 1,
                        }
                        *counter_ref.lock().unwrap() = (ok_counter, error_counter);
                    },

                    (false, false) => loop {
                        match tx.send_to(&packet, None).unwrap() {
                            Ok(_) => ok_counter += 1,
                            Err(_) => error_counter += 1,
                        }
                        *counter_ref.lock().unwrap() = (ok_counter, error_counter);
                    },
                }
            }
        });
    }

    let mut ok_sum: usize = 0;
    let mut ok_sum_previous: usize = 0;
    let mut error_sum: usize = 0;
    let mut error_sum_previous: usize = 0;
    loop {
        thread::sleep(time::Duration::from_secs(cli.watch as u64));
        for counter in &counters {
            ok_sum += counter.lock().unwrap().0;
            error_sum += counter.lock().unwrap().1;
        }
        println!(
            "OK: {}    Errors: {}    Speed: {:.3} Mbps    Speed: {:.3} pps",
            ok_sum - ok_sum_previous,
            error_sum - error_sum_previous,
            packet_length as f64 * (ok_sum as f64 - ok_sum_previous as f64) * 8.0 / 1024.0 / 1024.0 / cli.watch as f64,
            (ok_sum as f64 - ok_sum_previous as f64) / cli.watch as f64
        );
        ok_sum_previous = ok_sum;
        error_sum_previous = error_sum;
        ok_sum = 0;
        error_sum = 0;
    }
}
