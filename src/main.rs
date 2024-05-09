use clap::Parser;
use pnet::datalink::Channel::Ethernet;
use hex;
//use pnet::packet

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Packet as hex string
    #[arg(short, long, value_name = "PACKET")]
    packet: String,

    /// Interface name
    #[arg(short, long, value_name = "INTERFACE")]
    interface_name: String
}   

fn main() {
    let cli = Cli::parse();
    println!("{}", cli.packet);

    // Find the network interface with the provided name
    // TODO interface name mismatch error handling
    let interface = pnet::datalink::interfaces().into_iter()
                              .filter(|interface| interface.name == cli.interface_name)
                              .next()
                              .expect(&format!("Not found interface name {}", cli.interface_name));
    println!("{}", interface.name);
    
    let packet = hex::decode(&cli.packet).expect(&format!("Can not decode to hex packet string: \"{}\"", &cli.packet));
    
    // Create a new channel, dealing with layer 2 packets
    let (mut tx, _rx) = match pnet::datalink::channel(&interface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unhandled channel type"),
        Err(e) => panic!("An error occurred when creating the datalink channel: {}", e)
    };
    let mut ok_counter: usize = 0;
    let mut error_counter: usize = 0;
    for _i in 0..10000000 {
        match tx.send_to(&packet, None).unwrap() {
            Ok(_) => ok_counter+=1,
            Err(_) => error_counter +=1,
        }
    }
    println!("{} {}", ok_counter, error_counter);
}
