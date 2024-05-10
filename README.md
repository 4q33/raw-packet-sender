# raw-packet-sender
Small tool for sending raw ethernet packets

## Usage example

``` 
raw-packet-sender --packet 00..00 --interface dummy0 --threads 1 --sleep 1 --thread-number --packet-number
```

Where:

* packet — raw hex string of ethernet packet (in the example middle part of 62 zeroes is replaced by "..")
* interface — name of the ethernet inteface to which packets will be sent 
* thread — number of spawned threads (default 1)
* sleep — pause in seconds between counters checking
* thread-number — add thread number to the end of packet data
* packet-number — add packet number to the end of packet data (counts only successfully sent packets)

If activated thread-number and packet-number then thread number will be added before packet number in the way:

```
raw packet data + thread number + packet number
```

Size of thread number and packet number values is usize. Endiannes is inferred from system.