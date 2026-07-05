# Warning
This is my first Rust project, which is obvious when you inspect the code. This was used in a thesis for collecting data on different Connection Types.
More info will come if I get the time to add any.
# What it does
## Mullvad VPN
It currently creates automatic Mullvad VPN connections (Wireguard interfaces), communicating with their APIs.
## Tor
Uses an OnionMasq installation (currently) for creating Tor interfaces. Previous versions had a custom implementation of OnionMasq that didnt use a OnionMasq implementation. This is to be reimplementated. 

# IT WONT RUN
Not all code is exposed probably. To be specific, the current version is missing my custom OnionMasq implementation/program that allows binding to specific interfaces and etc.. 
