# TrafficStar Collector

A data collection tool developed for a master's thesis on connection-type 
fingerprinting of Tor and VPN users at Karlstad University (2025–2026).

## What it does

TrafficStar automatically creates Tor and Mullvad VPN (WireGuard) connections, 
generates upload traffic from a client to a server, and records server-side 
network traces using tcpdump. It was used to collect internet measurement 
datasets across four network access technologies (Ethernet, WiFi, 5G, Starlink).

## Thesis

TrafficStar: Connection-Type Fingerprinting of Tor Users — Reducing Anonymity 
by Detecting Connection Types. Karlstad University, 2026.

Diva link will be added once it's published on Diva; for now, check the included PDF. 

## TOR
The Tor implementation part is not currently exposed. Will update it once I get the time to do so :). 
Previous versions used a custom modified OnionMasq fork to make Tor interfaces. However, this exposed some problems with GuardManager that crashed the program in rare cases. So for now, it launches an OnionMasq binary as a program... 

## Mullvad VPN
The Mullvad VPN connections are generated automatically by communicating with their API, and create a new "Mullvad Device" for each VPN connection. Accounts are needed for this functionality. 
